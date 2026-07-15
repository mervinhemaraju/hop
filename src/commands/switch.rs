use std::path::Path;
use std::process::ExitCode;

use thiserror::Error;

use crate::adapters::gcloud_config::GcloudConfigSource;
use crate::adapters::gcloud_process::GcloudCli;
use crate::adapters::hop_files::HopFiles;
use crate::adapters::prompt::InquirePicker;
use crate::adapters::resource_manager::ResourceManagerApi;
use crate::commands::project_source::{
    ProjectSourceError, ProjectSourcePorts, Projects, obtain_projects,
};
use crate::commands::{
    EXIT_BAD_INPUT, EXIT_CANCELLED, EXIT_NOT_AUTHENTICATED, EXIT_NOT_INTERACTIVE,
};
use crate::core::error::{ApiError, AuthError, ConfigError, PromptError, ValidationError};
use crate::core::ports::{
    Authenticator, ConfigurationPicker, ConfigurationStore, Confirmer, ProjectCache, ProjectLister,
    ProjectPicker, SettingsStore, TokenProvider,
};
use crate::core::settings::Settings;
use crate::core::types::ProjectId;

// Any layer of the switch flow can fail; this keeps `?` working across all
// of them while the exit-code mapping stays in one place.
#[derive(Debug, Error)]
enum SwitchError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Prompt(#[from] PromptError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error("invalid project id: {0}")]
    InvalidProject(#[from] ValidationError),
}

impl From<ProjectSourceError> for SwitchError {
    fn from(err: ProjectSourceError) -> Self {
        match err {
            ProjectSourceError::Auth(err) => Self::Auth(err),
            ProjectSourceError::Prompt(err) => Self::Prompt(err),
            ProjectSourceError::Api(err) => Self::Api(err),
            ProjectSourceError::Config(err) => Self::Config(err),
        }
    }
}

impl SwitchError {
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::Config(ConfigError::UnknownConfiguration { .. }) | Self::InvalidProject(_) => {
                ExitCode::from(EXIT_BAD_INPUT)
            }
            Self::Prompt(PromptError::NotInteractive) => ExitCode::from(EXIT_NOT_INTERACTIVE),
            Self::Auth(AuthError::CredentialsInvalid { .. }) => {
                ExitCode::from(EXIT_NOT_AUTHENTICATED)
            }
            _ => ExitCode::FAILURE,
        }
    }
}

// All the ports the flow needs, as one injectable bundle. Trait objects
// rather than generics: eight type parameters would bury the logic, and a
// prompt-speed command has no use for monomorphization.
struct Ports<'a> {
    store: &'a dyn ConfigurationStore,
    config_picker: &'a dyn ConfigurationPicker,
    project_picker: &'a dyn ProjectPicker,
    confirmer: &'a dyn Confirmer,
    authenticator: &'a dyn Authenticator,
    tokens: &'a dyn TokenProvider,
    lister: &'a dyn ProjectLister,
    cache: &'a dyn ProjectCache,
}

struct Request<'a> {
    name: Option<&'a str>,
    project: Option<&'a str>,
    refresh: bool,
}

#[derive(Debug)]
enum FlowOutcome {
    /// The user cancelled the configuration picker.
    Cancelled,
    Done {
        configuration: String,
        switched: bool,
        project: ProjectOutcome,
    },
}

#[derive(Debug)]
enum ProjectOutcome {
    Set(ProjectId),
    /// User pressed Esc at the project picker; the configuration switch stands.
    Unchanged,
    /// User declined the re-auth prompt; nothing to list projects with.
    Declined,
    /// No terminal for the picker (and no --project given).
    NotInteractive,
    /// The target configuration has no account bound, so no project listing.
    NoAccount,
    /// The account can see no active projects.
    NoneAvailable,
}

/// Switch the active gcloud configuration and optionally its project.
pub fn run(name: Option<&str>, project: Option<&str>, refresh: bool) -> ExitCode {
    // Composition root: production adapters are chosen here and only here.
    let store = match GcloudConfigSource::new() {
        Ok(store) => store,
        Err(err) => return fail(&err.to_string()),
    };
    let hop_files = match HopFiles::new() {
        Ok(files) => files,
        Err(err) => return fail(&err.to_string()),
    };
    let settings = match hop_files.settings() {
        Ok(settings) => settings,
        Err(err) => return fail(&err.to_string()),
    };
    let picker = InquirePicker;
    let gcloud = GcloudCli::new(settings.browser.clone());
    let api = ResourceManagerApi::new();
    let ports = Ports {
        store: &store,
        config_picker: &picker,
        project_picker: &picker,
        confirmer: &picker,
        authenticator: &gcloud,
        tokens: &gcloud,
        lister: &api,
        cache: &hop_files,
    };
    let request = Request {
        name,
        project,
        refresh,
    };
    match switch_flow(&ports, &settings, &request) {
        Ok(FlowOutcome::Cancelled) => {
            eprintln!("cancelled");
            ExitCode::from(EXIT_CANCELLED)
        }
        Ok(FlowOutcome::Done {
            configuration,
            switched,
            project,
        }) => {
            if switched {
                eprintln!("switched to {configuration}");
            } else {
                eprintln!("already on {configuration}");
            }
            report_project(&project);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("hop switch: {err}");
            err.exit_code()
        }
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("hop switch: {message}");
    ExitCode::FAILURE
}

fn report_project(outcome: &ProjectOutcome) {
    match outcome {
        ProjectOutcome::Set(project) => eprintln!("project set to {project}"),
        ProjectOutcome::Unchanged => eprintln!("project unchanged"),
        ProjectOutcome::Declined => {
            eprintln!("project unchanged (run `hop login` when ready, then `hop switch` again)");
        }
        ProjectOutcome::NotInteractive => {
            eprintln!("project unchanged (no terminal for the picker; use --project <id>)");
        }
        ProjectOutcome::NoAccount => {
            eprintln!("configuration has no account; run `hop login` to attach one");
        }
        ProjectOutcome::NoneAvailable => {
            eprintln!("no active projects visible to this account");
        }
    }
}

// The testable body: configuration half, then project half.
fn switch_flow(
    ports: &Ports,
    settings: &Settings,
    request: &Request,
) -> Result<FlowOutcome, SwitchError> {
    let configurations = ports.store.list()?;
    if configurations.is_empty() {
        return Err(ConfigError::NoConfigurations.into());
    }
    let target = match request.name {
        Some(name) => name.to_string(),
        None => match ports
            .config_picker
            .pick("Switch to configuration:", &configurations)?
        {
            Some(choice) => choice,
            None => return Ok(FlowOutcome::Cancelled),
        },
    };
    let already_active = configurations
        .iter()
        .any(|c| c.name == target && c.is_active);
    if !already_active {
        ports.store.activate(&target)?;
    }

    let project = project_half(ports, settings, request, &target, &configurations)?;
    Ok(FlowOutcome::Done {
        configuration: target,
        switched: !already_active,
        project,
    })
}

fn project_half(
    ports: &Ports,
    settings: &Settings,
    request: &Request,
    target: &str,
    configurations: &[crate::core::context::Configuration],
) -> Result<ProjectOutcome, SwitchError> {
    // Explicit --project: validate, write, done; no listing, no network.
    if let Some(raw) = request.project {
        let project = ProjectId::new(raw)?;
        ports.store.set_project(target, &project)?;
        return Ok(ProjectOutcome::Set(project));
    }
    let target_configuration = configurations.iter().find(|c| c.name == target);
    let Some(account) = target_configuration.and_then(|c| c.account.clone()) else {
        return Ok(ProjectOutcome::NoAccount);
    };
    // Workforce configurations carry a login config; re-auth needs it.
    let login_config = target_configuration.and_then(|c| c.login_config_file.clone());
    // Without a terminal there is nothing to pick with, so avoid network
    // entirely unless the user explicitly asked to refresh the cache.
    let interactive = ports.project_picker.available();
    if !interactive && !request.refresh {
        return Ok(ProjectOutcome::NotInteractive);
    }
    let source = ProjectSourcePorts {
        tokens: ports.tokens,
        authenticator: ports.authenticator,
        confirmer: ports.confirmer,
        lister: ports.lister,
        cache: ports.cache,
    };
    let projects = match obtain_projects(
        &source,
        settings,
        &account,
        login_config.as_deref().map(Path::new),
        request.refresh,
    )? {
        Projects::List(projects) => projects,
        Projects::ReauthDeclined => return Ok(ProjectOutcome::Declined),
    };
    if !interactive {
        // --refresh from a script: cache updated, nothing to pick.
        return Ok(ProjectOutcome::NotInteractive);
    }
    if projects.is_empty() {
        return Ok(ProjectOutcome::NoneAvailable);
    }
    match ports.project_picker.pick("Switch to project:", &projects) {
        Ok(Some(project)) => {
            ports.store.set_project(target, &project)?;
            Ok(ProjectOutcome::Set(project))
        }
        Ok(None) => Ok(ProjectOutcome::Unchanged),
        Err(PromptError::NotInteractive) => Ok(ProjectOutcome::NotInteractive),
        Err(other) => Err(other.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::core::context::{Configuration, Project};
    use crate::core::settings::ReauthPolicy;
    use crate::core::types::{AccessToken, AccountEmail, ServiceAccount};

    struct FakeStore {
        configurations: Vec<Configuration>,
        activated: RefCell<Option<String>>,
        project_set: RefCell<Option<(String, String)>>,
    }

    impl FakeStore {
        fn with(entries: &[(&str, Option<&str>, bool)]) -> Self {
            Self {
                configurations: entries
                    .iter()
                    .map(|(name, account, is_active)| Configuration {
                        name: name.to_string(),
                        account: account.map(|a| AccountEmail::new(a).expect("valid test account")),
                        project: None,
                        is_active: *is_active,
                        login_config_file: None,
                    })
                    .collect(),
                activated: RefCell::new(None),
                project_set: RefCell::new(None),
            }
        }
    }

    impl ConfigurationStore for FakeStore {
        fn list(&self) -> Result<Vec<Configuration>, ConfigError> {
            Ok(self.configurations.clone())
        }

        fn activate(&self, name: &str) -> Result<(), ConfigError> {
            if !self.configurations.iter().any(|c| c.name == name) {
                return Err(ConfigError::UnknownConfiguration {
                    name: name.to_string(),
                });
            }
            *self.activated.borrow_mut() = Some(name.to_string());
            Ok(())
        }

        fn set_project(&self, name: &str, project: &ProjectId) -> Result<(), ConfigError> {
            *self.project_set.borrow_mut() = Some((name.to_string(), project.as_str().to_string()));
            Ok(())
        }

        fn set_impersonation(
            &self,
            _: &str,
            _: Option<&ServiceAccount>,
        ) -> Result<(), ConfigError> {
            panic!("set_impersonation must not be called from switch");
        }
    }

    struct FakeConfigPicker(Option<String>);

    impl ConfigurationPicker for FakeConfigPicker {
        fn pick(&self, _: &str, _: &[Configuration]) -> Result<Option<String>, PromptError> {
            Ok(self.0.clone())
        }
    }

    struct FakeProjectPicker {
        choice: Option<ProjectId>,
        available: bool,
    }

    impl ProjectPicker for FakeProjectPicker {
        fn available(&self) -> bool {
            self.available
        }

        fn pick(&self, _: &str, _: &[Project]) -> Result<Option<ProjectId>, PromptError> {
            if !self.available {
                return Err(PromptError::NotInteractive);
            }
            Ok(self.choice.clone())
        }
    }

    struct FakeConfirmer(Option<bool>);

    impl Confirmer for FakeConfirmer {
        fn confirm(&self, _: &str) -> Result<Option<bool>, PromptError> {
            Ok(self.0)
        }
    }

    struct PanickingConfirmer;

    impl Confirmer for PanickingConfirmer {
        fn confirm(&self, _: &str) -> Result<Option<bool>, PromptError> {
            panic!("confirmer must not be consulted");
        }
    }

    /// gcloud stand-in: TokenProvider and Authenticator in one, like the
    /// real GcloudCli. `login` repairs the credentials.
    struct FakeGcloud {
        invalid: RefCell<bool>,
        login_called: RefCell<bool>,
    }

    impl FakeGcloud {
        fn valid() -> Self {
            Self {
                invalid: RefCell::new(false),
                login_called: RefCell::new(false),
            }
        }

        fn expired() -> Self {
            Self {
                invalid: RefCell::new(true),
                login_called: RefCell::new(false),
            }
        }
    }

    impl TokenProvider for FakeGcloud {
        fn access_token(&self, account: &AccountEmail) -> Result<AccessToken, AuthError> {
            if *self.invalid.borrow() {
                return Err(AuthError::CredentialsInvalid {
                    account: account.as_str().to_string(),
                    detail: "expired".to_string(),
                });
            }
            Ok(AccessToken::new("fake-token-for-tests").expect("valid"))
        }
    }

    impl Authenticator for FakeGcloud {
        fn login(
            &self,
            _: Option<&AccountEmail>,
            _: bool,
            _: Option<&Path>,
        ) -> Result<(), AuthError> {
            *self.invalid.borrow_mut() = false;
            *self.login_called.borrow_mut() = true;
            Ok(())
        }
    }

    struct FakeLister(Vec<Project>);

    impl ProjectLister for FakeLister {
        fn list_projects(&self, _: &AccessToken) -> Result<Vec<Project>, ApiError> {
            Ok(self.0.clone())
        }
    }

    struct PanickingLister;

    impl ProjectLister for PanickingLister {
        fn list_projects(&self, _: &AccessToken) -> Result<Vec<Project>, ApiError> {
            panic!("the API must not be called");
        }
    }

    struct FakeCache {
        preloaded: Option<Vec<Project>>,
        stored: RefCell<Option<Vec<Project>>>,
    }

    impl FakeCache {
        fn empty() -> Self {
            Self {
                preloaded: None,
                stored: RefCell::new(None),
            }
        }

        fn with(projects: Vec<Project>) -> Self {
            Self {
                preloaded: Some(projects),
                stored: RefCell::new(None),
            }
        }
    }

    impl ProjectCache for FakeCache {
        fn cached_projects(&self, _: &AccountEmail) -> Result<Option<Vec<Project>>, ConfigError> {
            Ok(self.preloaded.clone())
        }

        fn store_projects(
            &self,
            _: &AccountEmail,
            projects: &[Project],
        ) -> Result<(), ConfigError> {
            *self.stored.borrow_mut() = Some(projects.to_vec());
            Ok(())
        }
    }

    fn project(id: &str) -> Project {
        Project {
            id: ProjectId::new(id).expect("valid test project"),
            display_name: None,
        }
    }

    fn request<'a>(name: Option<&'a str>, project: Option<&'a str>, refresh: bool) -> Request<'a> {
        Request {
            name,
            project,
            refresh,
        }
    }

    struct Fixture {
        store: FakeStore,
        config_picker: FakeConfigPicker,
        project_picker: FakeProjectPicker,
        confirmer: FakeConfirmer,
        gcloud: FakeGcloud,
        lister: FakeLister,
        cache: FakeCache,
    }

    impl Fixture {
        fn ports(&self) -> Ports<'_> {
            Ports {
                store: &self.store,
                config_picker: &self.config_picker,
                project_picker: &self.project_picker,
                confirmer: &self.confirmer,
                authenticator: &self.gcloud,
                tokens: &self.gcloud,
                lister: &self.lister,
                cache: &self.cache,
            }
        }
    }

    fn fixture() -> Fixture {
        Fixture {
            store: FakeStore::with(&[
                ("default", Some("dev@example.com"), true),
                ("work", Some("dev@example.com"), false),
            ]),
            config_picker: FakeConfigPicker(None),
            project_picker: FakeProjectPicker {
                choice: Some(ProjectId::new("my-project-123").expect("valid")),
                available: true,
            },
            confirmer: FakeConfirmer(None),
            gcloud: FakeGcloud::valid(),
            lister: FakeLister(vec![project("my-project-123")]),
            cache: FakeCache::empty(),
        }
    }

    #[test]
    fn full_interactive_switch_from_cache_needs_no_token() {
        // arrange: expired credentials prove the cache short-circuits auth
        let mut fix = fixture();
        fix.config_picker = FakeConfigPicker(Some("work".to_string()));
        fix.gcloud = FakeGcloud::expired();
        fix.cache = FakeCache::with(vec![project("my-project-123")]);
        // act
        let outcome = switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(None, None, false),
        )
        .expect("flow failed");
        // assert
        assert!(matches!(
            outcome,
            FlowOutcome::Done {
                switched: true,
                project: ProjectOutcome::Set(_),
                ..
            }
        ));
        assert_eq!(fix.store.activated.borrow().as_deref(), Some("work"));
        assert_eq!(
            fix.store.project_set.borrow().clone(),
            Some(("work".to_string(), "my-project-123".to_string()))
        );
    }

    #[test]
    fn refresh_bypasses_the_cache_and_stores_the_fetch() {
        // arrange: stale cache, fresh listing
        let mut fix = fixture();
        fix.cache = FakeCache::with(vec![project("stale-project")]);
        fix.lister = FakeLister(vec![project("my-project-123")]);
        // act
        switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(Some("work"), None, true),
        )
        .expect("flow failed");
        // assert
        assert_eq!(
            fix.cache.stored.borrow().clone(),
            Some(vec![project("my-project-123")])
        );
    }

    #[test]
    fn expired_credentials_with_prompt_yes_reauths_and_continues() {
        // arrange
        let mut fix = fixture();
        fix.gcloud = FakeGcloud::expired();
        fix.confirmer = FakeConfirmer(Some(true));
        // act
        let outcome = switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(Some("work"), None, false),
        )
        .expect("flow failed");
        // assert
        assert!(*fix.gcloud.login_called.borrow());
        assert!(matches!(
            outcome,
            FlowOutcome::Done {
                project: ProjectOutcome::Set(_),
                ..
            }
        ));
    }

    #[test]
    fn expired_credentials_with_prompt_no_leaves_project_unchanged() {
        // arrange
        let mut fix = fixture();
        fix.gcloud = FakeGcloud::expired();
        fix.confirmer = FakeConfirmer(Some(false));
        // act
        let outcome = switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(Some("work"), None, false),
        )
        .expect("flow failed");
        // assert
        assert!(!*fix.gcloud.login_called.borrow());
        assert!(matches!(
            outcome,
            FlowOutcome::Done {
                switched: true,
                project: ProjectOutcome::Declined,
                ..
            }
        ));
    }

    #[test]
    fn expired_credentials_with_policy_off_fail_without_prompting() {
        // arrange
        let mut fix = fixture();
        fix.gcloud = FakeGcloud::expired();
        let settings = Settings {
            reauth: ReauthPolicy::Off,
            ..Settings::default()
        };
        let ports = Ports {
            confirmer: &PanickingConfirmer,
            ..fix.ports()
        };
        // act
        let err = switch_flow(&ports, &settings, &request(Some("work"), None, false))
            .expect_err("expired credentials were accepted");
        // assert
        assert!(matches!(
            err,
            SwitchError::Auth(AuthError::CredentialsInvalid { .. })
        ));
    }

    #[test]
    fn expired_credentials_with_policy_auto_reauth_without_prompting() {
        // arrange
        let mut fix = fixture();
        fix.gcloud = FakeGcloud::expired();
        let settings = Settings {
            reauth: ReauthPolicy::Auto,
            ..Settings::default()
        };
        let ports = Ports {
            confirmer: &PanickingConfirmer,
            ..fix.ports()
        };
        // act
        let outcome = switch_flow(&ports, &settings, &request(Some("work"), None, false))
            .expect("flow failed");
        // assert
        assert!(*fix.gcloud.login_called.borrow());
        assert!(matches!(
            outcome,
            FlowOutcome::Done {
                project: ProjectOutcome::Set(_),
                ..
            }
        ));
    }

    #[test]
    fn explicit_project_flag_touches_no_network() {
        // arrange: everything network-ish panics if consulted
        let fix = fixture();
        let ports = Ports {
            lister: &PanickingLister,
            confirmer: &PanickingConfirmer,
            ..fix.ports()
        };
        // act
        let outcome = switch_flow(
            &ports,
            &Settings::default(),
            &request(Some("work"), Some("other-project-456"), false),
        )
        .expect("flow failed");
        // assert
        assert!(matches!(
            outcome,
            FlowOutcome::Done {
                project: ProjectOutcome::Set(_),
                ..
            }
        ));
        assert_eq!(
            fix.store.project_set.borrow().clone(),
            Some(("work".to_string(), "other-project-456".to_string()))
        );
    }

    #[test]
    fn an_invalid_project_flag_is_bad_input() {
        // arrange
        let fix = fixture();
        // act
        let err = switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(Some("work"), Some("bad project"), false),
        )
        .expect_err("accepted a project id with a space");
        // assert
        assert!(matches!(err, SwitchError::InvalidProject(_)));
    }

    #[test]
    fn escaping_the_project_picker_keeps_the_switch() {
        // arrange
        let mut fix = fixture();
        fix.project_picker = FakeProjectPicker {
            choice: None,
            available: true,
        };
        fix.cache = FakeCache::with(vec![project("my-project-123")]);
        // act
        let outcome = switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(Some("work"), None, false),
        )
        .expect("flow failed");
        // assert
        assert!(matches!(
            outcome,
            FlowOutcome::Done {
                switched: true,
                project: ProjectOutcome::Unchanged,
                ..
            }
        ));
        assert_eq!(fix.store.project_set.borrow().clone(), None);
    }

    #[test]
    fn a_configuration_without_account_skips_the_project_half() {
        // arrange
        let mut fix = fixture();
        fix.store = FakeStore::with(&[("default", None, true), ("work", None, false)]);
        let ports = Ports {
            lister: &PanickingLister,
            ..fix.ports()
        };
        // act
        let outcome = switch_flow(
            &ports,
            &Settings::default(),
            &request(Some("work"), None, false),
        )
        .expect("flow failed");
        // assert
        assert!(matches!(
            outcome,
            FlowOutcome::Done {
                project: ProjectOutcome::NoAccount,
                ..
            }
        ));
    }

    #[test]
    fn no_terminal_and_no_refresh_means_no_network() {
        // arrange
        let mut fix = fixture();
        fix.project_picker = FakeProjectPicker {
            choice: None,
            available: false,
        };
        let ports = Ports {
            lister: &PanickingLister,
            ..fix.ports()
        };
        // act
        let outcome = switch_flow(
            &ports,
            &Settings::default(),
            &request(Some("work"), None, false),
        )
        .expect("flow failed");
        // assert
        assert!(matches!(
            outcome,
            FlowOutcome::Done {
                switched: true,
                project: ProjectOutcome::NotInteractive,
                ..
            }
        ));
    }

    #[test]
    fn cancelling_the_configuration_picker_cancels_everything() {
        // arrange
        let fix = fixture();
        // act
        let outcome = switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(None, None, false),
        )
        .expect("flow failed");
        // assert
        assert!(matches!(outcome, FlowOutcome::Cancelled));
        assert_eq!(fix.store.activated.borrow().as_deref(), None);
    }

    #[test]
    fn an_unknown_name_maps_to_bad_input() {
        // arrange
        let fix = fixture();
        // act
        let err = switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(Some("nope"), None, false),
        )
        .expect_err("activated a ghost");
        // assert
        assert!(matches!(
            err,
            SwitchError::Config(ConfigError::UnknownConfiguration { .. })
        ));
    }

    #[test]
    fn no_configurations_at_all_is_an_error() {
        // arrange
        let mut fix = fixture();
        fix.store = FakeStore::with(&[]);
        // act
        let err = switch_flow(
            &fix.ports(),
            &Settings::default(),
            &request(None, None, false),
        )
        .expect_err("empty store accepted");
        // assert
        assert!(matches!(
            err,
            SwitchError::Config(ConfigError::NoConfigurations)
        ));
    }
}
