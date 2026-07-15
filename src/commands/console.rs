use std::env;
use std::path::Path;
use std::process::ExitCode;

use thiserror::Error;

use crate::adapters::browser::{CustomBrowser, SystemBrowser};
use crate::adapters::gcloud_config::GcloudConfigSource;
use crate::adapters::gcloud_process::GcloudCli;
use crate::adapters::hop_files::HopFiles;
use crate::adapters::login_config::load_workforce_provider;
use crate::adapters::prompt::InquirePicker;
use crate::adapters::resource_manager::ResourceManagerApi;
use crate::commands::project_source::{
    ProjectSourceError, ProjectSourcePorts, Projects, obtain_projects,
};
use crate::commands::{
    EXIT_BAD_INPUT, EXIT_CANCELLED, EXIT_NOT_AUTHENTICATED, EXIT_NOT_INTERACTIVE,
};
use crate::core::console::{console_url, federated_console_url};
use crate::core::context::{Configuration, Context, IdentityKind};
use crate::core::error::{ApiError, AuthError, ConfigError, PromptError, ValidationError};
use crate::core::ports::{
    Authenticator, BrowserOpener, ConfigurationPicker, ConfigurationStore, Confirmer, ProjectCache,
    ProjectLister, ProjectPicker, SettingsStore, TokenProvider,
};
use crate::core::settings::{Settings, effective_browser};
use crate::core::types::ProjectId;

// The console flow can fail at the config, auth, API, prompt, or validation
// layer; one error type keeps `?` working and the exit-code mapping in one place.
#[derive(Debug, Error)]
enum ConsoleError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error(transparent)]
    Prompt(#[from] PromptError),
    #[error("invalid project id: {0}")]
    InvalidProject(#[from] ValidationError),
    #[error(
        "no active gcloud configuration; pass a configuration name or activate one with `hop switch`"
    )]
    NoActiveConfiguration,
    #[error(
        "no project in the chosen configuration; pass --project <id> or set one with `hop switch`"
    )]
    NoProject,
    #[error("the chosen configuration has no account; run `hop login` or pass --project <id>")]
    NoAccount,
    #[error("no active projects visible to this account; pass --project <id> or run `hop switch`")]
    NoProjectsAvailable,
}

impl From<ProjectSourceError> for ConsoleError {
    fn from(err: ProjectSourceError) -> Self {
        match err {
            ProjectSourceError::Auth(err) => Self::Auth(err),
            ProjectSourceError::Prompt(err) => Self::Prompt(err),
            ProjectSourceError::Api(err) => Self::Api(err),
            ProjectSourceError::Config(err) => Self::Config(err),
        }
    }
}

impl ConsoleError {
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::InvalidProject(_) | Self::Config(ConfigError::UnknownConfiguration { .. }) => {
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

// All the ports the flow needs, as one injectable bundle (composition-root
// pattern, rules/architecture.md).
struct Ports<'a> {
    store: &'a dyn ConfigurationStore,
    config_picker: &'a dyn ConfigurationPicker,
    project_picker: &'a dyn ProjectPicker,
    tokens: &'a dyn TokenProvider,
    authenticator: &'a dyn Authenticator,
    confirmer: &'a dyn Confirmer,
    lister: &'a dyn ProjectLister,
    cache: &'a dyn ProjectCache,
}

struct Request<'a> {
    name: Option<&'a str>,
    project: Option<&'a str>,
    refresh: bool,
    show_principal: bool,
}

#[derive(Debug)]
enum ConsoleOutcome {
    /// Open the console for `project`; identity comes from `context`.
    Open {
        context: Context,
        project: ProjectId,
    },
    /// The user cancelled a picker or declined to re-authenticate.
    Cancelled,
}

/// Open the GCP console in the browser for a chosen (or given) project.
pub fn run(
    name: Option<&str>,
    project: Option<&str>,
    url_only: bool,
    refresh: bool,
    show_principal: bool,
) -> ExitCode {
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
        tokens: &gcloud,
        authenticator: &gcloud,
        confirmer: &picker,
        lister: &api,
        cache: &hop_files,
    };
    let request = Request {
        name,
        project,
        refresh,
        show_principal,
    };
    match console_flow(&ports, &settings, &request) {
        Ok(ConsoleOutcome::Cancelled) => {
            eprintln!("cancelled");
            ExitCode::from(EXIT_CANCELLED)
        }
        Ok(ConsoleOutcome::Open { context, project }) => {
            open_console(&context, &project, url_only, &settings)
        }
        Err(err) => {
            eprintln!("hop console: {err}");
            err.exit_code()
        }
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("hop console: {message}");
    ExitCode::FAILURE
}

// The testable body: resolve which configuration and project to open, leaving
// the browser side effect to `open_console`. Console never mutates gcloud
// state; it only reads a configuration's account and identity.
fn console_flow(
    ports: &Ports,
    settings: &Settings,
    request: &Request,
) -> Result<ConsoleOutcome, ConsoleError> {
    let configurations = ports.store.list()?;
    if configurations.is_empty() {
        return Err(ConfigError::NoConfigurations.into());
    }

    // The same InquirePicker backs both pickers, so this single check reflects
    // whether any interactive prompt is possible at all.
    let interactive = ports.project_picker.available();

    // Resolve which configuration's context to open the console with.
    let target = match request.name {
        Some(name) => find_named(&configurations, name)?,
        None if interactive => {
            match ports.config_picker.pick(
                "Open configuration:",
                request.show_principal,
                &configurations,
            )? {
                Some(name) => find_named(&configurations, &name)?,
                None => return Ok(ConsoleOutcome::Cancelled),
            }
        }
        // No terminal to pick on: open the currently active configuration.
        None => active_configuration(&configurations)?,
    };
    let context = context_from(&target);

    // Explicit --project: validate and use it, no listing and no network.
    if let Some(raw) = request.project {
        let project = ProjectId::new(raw)?;
        return Ok(ConsoleOutcome::Open { context, project });
    }

    // No terminal for the project picker: open the configuration's own project.
    if !interactive {
        return open_or(context, ConsoleError::NoProject);
    }

    // Listing projects needs an account to authenticate as; without one, fall
    // back to the configuration's project rather than failing outright.
    let Some(account) = context.account.clone() else {
        return open_or(context, ConsoleError::NoAccount);
    };

    let login_config = context.login_config_file.clone();
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
        Projects::ReauthDeclined => return Ok(ConsoleOutcome::Cancelled),
    };
    if projects.is_empty() {
        return open_or(context, ConsoleError::NoProjectsAvailable);
    }
    match ports.project_picker.pick("Open project:", &projects) {
        Ok(Some(project)) => Ok(ConsoleOutcome::Open { context, project }),
        Ok(None) => Ok(ConsoleOutcome::Cancelled),
        // The terminal vanished mid-pick; fall back to the config's project.
        Err(PromptError::NotInteractive) => open_or(context, ConsoleError::NoProject),
        Err(other) => Err(other.into()),
    }
}

fn find_named(configurations: &[Configuration], name: &str) -> Result<Configuration, ConsoleError> {
    configurations
        .iter()
        .find(|c| c.name == name)
        .cloned()
        .ok_or_else(|| {
            ConsoleError::Config(ConfigError::UnknownConfiguration {
                name: name.to_string(),
            })
        })
}

fn active_configuration(configurations: &[Configuration]) -> Result<Configuration, ConsoleError> {
    configurations
        .iter()
        .find(|c| c.is_active)
        .cloned()
        .ok_or(ConsoleError::NoActiveConfiguration)
}

// Console reads a configuration's identity without ever activating it, so it
// builds a Context directly rather than through the active-context adapter.
fn context_from(configuration: &Configuration) -> Context {
    Context {
        name: configuration.name.clone(),
        account: configuration.account.clone(),
        project: configuration.project.clone(),
        impersonation: None,
        login_config_file: configuration.login_config_file.clone(),
    }
}

// Open the configuration's own project, or report `missing` if it has none.
fn open_or(context: Context, missing: ConsoleError) -> Result<ConsoleOutcome, ConsoleError> {
    match context.project.clone() {
        Some(project) => Ok(ConsoleOutcome::Open { context, project }),
        None => Err(missing),
    }
}

// Build the console URL for `project` under `context`'s identity, then either
// print it (machine-consumable, stdout) or open it in the browser.
fn open_console(
    context: &Context,
    project: &ProjectId,
    url_only: bool,
    settings: &Settings,
) -> ExitCode {
    // Workforce sessions go through the federated console sign-in URL; the
    // standard console URL would prompt for a Google account they lack.
    let url = match context.identity() {
        IdentityKind::Workforce => {
            let Some(raw_path) = context.login_config_file.as_deref() else {
                return fail(
                    "workforce session, but the configuration has no auth/login_config_file property; re-run `gcloud iam workforce-pools create-login-config <provider> --activate`",
                );
            };
            match load_workforce_provider(Path::new(raw_path)) {
                Ok(provider) => federated_console_url(&provider, Some(project)),
                Err(err) => return fail(&err.to_string()),
            }
        }
        IdentityKind::Google => console_url(project, context.account.as_ref()),
    };
    if url_only {
        // stdout on purpose: this is machine-consumable output
        // (rules/cli-ux.md), e.g. `open "$(hop console --url)"`.
        println!("{url}");
        return ExitCode::SUCCESS;
    }
    eprintln!("opening {url}");
    // BROWSER env var wins over the setting; neither means the OS default.
    let opened = match effective_browser(
        env::var_os("BROWSER").as_deref(),
        settings.browser.as_deref(),
    ) {
        Some(command) => CustomBrowser::new(command).open_url(&url),
        None => SystemBrowser.open_url(&url),
    };
    match opened {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => fail(&err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::core::context::Project;
    use crate::core::types::{AccessToken, AccountEmail, ServiceAccount};

    struct FakeStore(Vec<Configuration>);

    impl FakeStore {
        fn with(entries: &[(&str, Option<&str>, Option<&str>, bool)]) -> Self {
            Self(
                entries
                    .iter()
                    .map(|(name, account, project, is_active)| Configuration {
                        name: name.to_string(),
                        account: account.map(|a| AccountEmail::new(a).expect("valid test account")),
                        project: project.map(|p| ProjectId::new(p).expect("valid test project")),
                        is_active: *is_active,
                        login_config_file: None,
                    })
                    .collect(),
            )
        }
    }

    // Console must never mutate gcloud state; the write methods prove it by
    // panicking if the flow ever reaches for them.
    impl ConfigurationStore for FakeStore {
        fn list(&self) -> Result<Vec<Configuration>, ConfigError> {
            Ok(self.0.clone())
        }

        fn activate(&self, _: &str) -> Result<(), ConfigError> {
            panic!("console must not activate a configuration");
        }

        fn set_project(&self, _: &str, _: &ProjectId) -> Result<(), ConfigError> {
            panic!("console must not set a project");
        }

        fn set_impersonation(
            &self,
            _: &str,
            _: Option<&ServiceAccount>,
        ) -> Result<(), ConfigError> {
            panic!("console must not set impersonation");
        }
    }

    struct FakeConfigPicker(Option<String>);

    impl ConfigurationPicker for FakeConfigPicker {
        fn pick(
            &self,
            _: &str,
            _: bool,
            _: &[Configuration],
        ) -> Result<Option<String>, PromptError> {
            Ok(self.0.clone())
        }
    }

    struct PanickingConfigPicker;

    impl ConfigurationPicker for PanickingConfigPicker {
        fn pick(
            &self,
            _: &str,
            _: bool,
            _: &[Configuration],
        ) -> Result<Option<String>, PromptError> {
            panic!("the configuration picker must not be consulted");
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

    struct PanickingProjectPicker;

    impl ProjectPicker for PanickingProjectPicker {
        fn available(&self) -> bool {
            true
        }

        fn pick(&self, _: &str, _: &[Project]) -> Result<Option<ProjectId>, PromptError> {
            panic!("the project picker must not be consulted");
        }
    }

    // gcloud stand-in: TokenProvider and Authenticator in one, like GcloudCli.
    struct FakeGcloud {
        invalid: RefCell<bool>,
    }

    impl FakeGcloud {
        fn valid() -> Self {
            Self {
                invalid: RefCell::new(false),
            }
        }

        fn expired() -> Self {
            Self {
                invalid: RefCell::new(true),
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
            Ok(())
        }
    }

    struct FakeConfirmer(Option<bool>);

    impl Confirmer for FakeConfirmer {
        fn confirm(&self, _: &str) -> Result<Option<bool>, PromptError> {
            Ok(self.0)
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

    struct FakeCache(Option<Vec<Project>>);

    impl ProjectCache for FakeCache {
        fn cached_projects(&self, _: &AccountEmail) -> Result<Option<Vec<Project>>, ConfigError> {
            Ok(self.0.clone())
        }

        fn store_projects(&self, _: &AccountEmail, _: &[Project]) -> Result<(), ConfigError> {
            Ok(())
        }
    }

    fn project(id: &str) -> Project {
        Project {
            id: ProjectId::new(id).expect("valid test project"),
            display_name: None,
        }
    }

    struct Fixture {
        store: FakeStore,
        config_picker: FakeConfigPicker,
        project_picker: FakeProjectPicker,
        gcloud: FakeGcloud,
        confirmer: FakeConfirmer,
        lister: FakeLister,
        cache: FakeCache,
    }

    impl Fixture {
        fn ports(&self) -> Ports<'_> {
            Ports {
                store: &self.store,
                config_picker: &self.config_picker,
                project_picker: &self.project_picker,
                tokens: &self.gcloud,
                authenticator: &self.gcloud,
                confirmer: &self.confirmer,
                lister: &self.lister,
                cache: &self.cache,
            }
        }
    }

    fn fixture() -> Fixture {
        Fixture {
            store: FakeStore::with(&[
                (
                    "default",
                    Some("dev@example.com"),
                    Some("active-project-1"),
                    true,
                ),
                ("work", Some("dev@example.com"), Some("work-project"), false),
            ]),
            config_picker: FakeConfigPicker(Some("work".to_string())),
            project_picker: FakeProjectPicker {
                choice: Some(ProjectId::new("picked-project-2").expect("valid")),
                available: true,
            },
            gcloud: FakeGcloud::valid(),
            confirmer: FakeConfirmer(None),
            lister: FakeLister(vec![project("picked-project-2")]),
            cache: FakeCache(Some(vec![project("picked-project-2")])),
        }
    }

    fn request<'a>(name: Option<&'a str>, project: Option<&'a str>, refresh: bool) -> Request<'a> {
        Request {
            name,
            project,
            refresh,
            show_principal: false,
        }
    }

    fn opened(outcome: &ConsoleOutcome) -> (&str, &str) {
        match outcome {
            ConsoleOutcome::Open { context, project } => (context.name.as_str(), project.as_str()),
            ConsoleOutcome::Cancelled => panic!("expected an Open outcome, got Cancelled"),
        }
    }

    #[test]
    fn explicit_name_and_project_open_without_pickers_or_network() {
        // arrange: any picker or listing would panic if consulted
        let fix = fixture();
        let ports = Ports {
            config_picker: &PanickingConfigPicker,
            project_picker: &PanickingProjectPicker,
            lister: &PanickingLister,
            ..fix.ports()
        };
        // act
        let outcome = console_flow(
            &ports,
            &Settings::default(),
            &request(Some("work"), Some("flag-project-9"), false),
        )
        .expect("flow failed");
        // assert
        assert_eq!(opened(&outcome), ("work", "flag-project-9"));
    }

    #[test]
    fn an_unknown_configuration_name_is_bad_input() {
        // arrange
        let fix = fixture();
        // act
        let err = console_flow(
            &fix.ports(),
            &Settings::default(),
            &request(Some("ghost"), None, false),
        )
        .expect_err("accepted an unknown configuration");
        // assert
        assert!(matches!(
            err,
            ConsoleError::Config(ConfigError::UnknownConfiguration { .. })
        ));
    }

    #[test]
    fn an_invalid_project_flag_is_bad_input() {
        // arrange
        let fix = fixture();
        // act
        let err = console_flow(
            &fix.ports(),
            &Settings::default(),
            &request(Some("work"), Some("bad project"), false),
        )
        .expect_err("accepted a project id with a space");
        // assert
        assert!(matches!(err, ConsoleError::InvalidProject(_)));
    }

    #[test]
    fn full_interactive_flow_opens_the_picked_configuration_and_project() {
        // arrange: defaults pick "work" then "picked-project-2"
        let fix = fixture();
        // act
        let outcome = console_flow(
            &fix.ports(),
            &Settings::default(),
            &request(None, None, false),
        )
        .expect("flow failed");
        // assert
        assert_eq!(opened(&outcome), ("work", "picked-project-2"));
    }

    #[test]
    fn cancelling_the_configuration_picker_cancels_everything() {
        // arrange
        let mut fix = fixture();
        fix.config_picker = FakeConfigPicker(None);
        let ports = Ports {
            lister: &PanickingLister,
            ..fix.ports()
        };
        // act
        let outcome = console_flow(&ports, &Settings::default(), &request(None, None, false))
            .expect("flow failed");
        // assert
        assert!(matches!(outcome, ConsoleOutcome::Cancelled));
    }

    #[test]
    fn escaping_the_project_picker_cancels() {
        // arrange
        let mut fix = fixture();
        fix.project_picker = FakeProjectPicker {
            choice: None,
            available: true,
        };
        // act
        let outcome = console_flow(
            &fix.ports(),
            &Settings::default(),
            &request(None, None, false),
        )
        .expect("flow failed");
        // assert
        assert!(matches!(outcome, ConsoleOutcome::Cancelled));
    }

    #[test]
    fn declining_reauth_cancels() {
        // arrange: expired creds, empty cache forces a token, prompt answered no
        let mut fix = fixture();
        fix.gcloud = FakeGcloud::expired();
        fix.confirmer = FakeConfirmer(Some(false));
        fix.cache = FakeCache(None);
        // act
        let outcome = console_flow(
            &fix.ports(),
            &Settings::default(),
            &request(None, None, false),
        )
        .expect("flow failed");
        // assert
        assert!(matches!(outcome, ConsoleOutcome::Cancelled));
    }

    #[test]
    fn non_interactive_opens_the_active_configuration_project() {
        // arrange: no terminal, so no config or project picker, no listing
        let mut fix = fixture();
        fix.project_picker = FakeProjectPicker {
            choice: None,
            available: false,
        };
        let ports = Ports {
            config_picker: &PanickingConfigPicker,
            lister: &PanickingLister,
            ..fix.ports()
        };
        // act
        let outcome = console_flow(&ports, &Settings::default(), &request(None, None, false))
            .expect("flow failed");
        // assert: "default" is the active configuration
        assert_eq!(opened(&outcome), ("default", "active-project-1"));
    }

    #[test]
    fn non_interactive_without_an_active_project_is_an_error() {
        // arrange: active configuration has no project set
        let mut fix = fixture();
        fix.store = FakeStore::with(&[("default", Some("dev@example.com"), None, true)]);
        fix.project_picker = FakeProjectPicker {
            choice: None,
            available: false,
        };
        // act
        let err = console_flow(
            &fix.ports(),
            &Settings::default(),
            &request(None, None, false),
        )
        .expect_err("opened without a project");
        // assert
        assert!(matches!(err, ConsoleError::NoProject));
    }

    #[test]
    fn an_empty_project_list_falls_back_to_the_configuration_project() {
        // arrange: nothing to pick, but the chosen config has a project
        let mut fix = fixture();
        fix.cache = FakeCache(None);
        fix.lister = FakeLister(vec![]);
        // act
        let outcome = console_flow(
            &fix.ports(),
            &Settings::default(),
            &request(None, None, false),
        )
        .expect("flow failed");
        // assert: config picker chose "work", whose project is "work-project"
        assert_eq!(opened(&outcome), ("work", "work-project"));
    }

    #[test]
    fn interactive_pick_serves_from_cache_without_a_token() {
        // arrange: expired creds prove the cache short-circuits auth
        let mut fix = fixture();
        fix.gcloud = FakeGcloud::expired();
        let ports = Ports {
            lister: &PanickingLister,
            ..fix.ports()
        };
        // act
        let outcome = console_flow(&ports, &Settings::default(), &request(None, None, false))
            .expect("flow failed");
        // assert
        assert_eq!(opened(&outcome), ("work", "picked-project-2"));
    }
}
