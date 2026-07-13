use std::path::Path;
use std::process::ExitCode;

use thiserror::Error;

use crate::adapters::gcloud_config::GcloudConfigSource;
use crate::adapters::gcloud_process::GcloudCli;
use crate::adapters::hop_files::HopFiles;
use crate::adapters::iam_api::IamApi;
use crate::adapters::prompt::InquirePicker;
use crate::commands::auth_flow::{AuthFlow, AuthFlowError, TokenOutcome, token_with_reauth};
use crate::commands::{
    EXIT_BAD_INPUT, EXIT_CANCELLED, EXIT_NOT_AUTHENTICATED, EXIT_NOT_INTERACTIVE,
    EXIT_PERMISSION_DENIED,
};
use crate::core::context::Context;
use crate::core::error::{ApiError, AuthError, ConfigError, PromptError, ValidationError};
use crate::core::ports::{
    ConfigurationStore, ContextSource, ImpersonationVerifier, ServiceAccountLister,
    ServiceAccountPicker, SettingsStore,
};
use crate::core::settings::Settings;
use crate::core::types::ServiceAccount;

#[derive(Debug, Error)]
enum ImpersonateError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Prompt(#[from] PromptError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error("invalid service account: {0}")]
    InvalidServiceAccount(#[from] ValidationError),
    #[error("the active configuration has no account; run `hop login` first")]
    NoAccount,
    #[error(
        "the active configuration has no project (service accounts are listed per project); run `hop switch` first or pass the service account directly"
    )]
    NoProject,
    #[error("no enabled service accounts in project {0}")]
    NoServiceAccounts(String),
}

impl From<AuthFlowError> for ImpersonateError {
    fn from(err: AuthFlowError) -> Self {
        match err {
            AuthFlowError::Auth(err) => Self::Auth(err),
            AuthFlowError::Prompt(err) => Self::Prompt(err),
        }
    }
}

impl ImpersonateError {
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::InvalidServiceAccount(_) => ExitCode::from(EXIT_BAD_INPUT),
            Self::Prompt(PromptError::NotInteractive) => ExitCode::from(EXIT_NOT_INTERACTIVE),
            Self::Auth(AuthError::CredentialsInvalid { .. }) => {
                ExitCode::from(EXIT_NOT_AUTHENTICATED)
            }
            // 403 from the verify-mint: authenticated fine, but lacking the
            // token-creator role on the target service account.
            Self::Api(ApiError::Status(403)) => ExitCode::from(EXIT_PERMISSION_DENIED),
            _ => ExitCode::FAILURE,
        }
    }
}

struct Ports<'a> {
    store: &'a dyn ConfigurationStore,
    lister: &'a dyn ServiceAccountLister,
    verifier: &'a dyn ImpersonationVerifier,
    picker: &'a dyn ServiceAccountPicker,
    auth: AuthFlow<'a>,
}

/// Impersonate a service account on the active configuration (or stop).
pub fn run(service_account: Option<&str>, clear: bool) -> ExitCode {
    // Composition root: production adapters are chosen here and only here.
    let source = match GcloudConfigSource::new() {
        Ok(source) => source,
        Err(err) => return fail(&err.to_string()),
    };
    let context = match source.active_context() {
        Ok(context) => context,
        Err(err) => return fail(&err.to_string()),
    };
    if clear {
        // Clearing is local-only: no token, no network.
        return match source.set_impersonation(&context.name, None) {
            Ok(()) => {
                eprintln!("impersonation cleared on {}", context.name);
                ExitCode::SUCCESS
            }
            Err(err) => fail(&err.to_string()),
        };
    }
    let hop_files = match HopFiles::new() {
        Ok(files) => files,
        Err(err) => return fail(&err.to_string()),
    };
    let settings = match hop_files.settings() {
        Ok(settings) => settings,
        Err(err) => return fail(&err.to_string()),
    };
    let picker = InquirePicker;
    let gcloud = GcloudCli;
    let api = IamApi::new();
    let ports = Ports {
        store: &source,
        lister: &api,
        verifier: &api,
        picker: &picker,
        auth: AuthFlow {
            tokens: &gcloud,
            authenticator: &gcloud,
            confirmer: &picker,
            // Workforce sessions re-auth through their login config.
            login_config: context.login_config_file.as_deref().map(Path::new),
        },
    };
    match impersonate_flow(&ports, &context, settings, service_account) {
        Ok(Some(sa)) => {
            eprintln!("impersonating {sa} on {} (verified)", context.name);
            ExitCode::SUCCESS
        }
        Ok(None) => {
            eprintln!("cancelled");
            ExitCode::from(EXIT_CANCELLED)
        }
        Err(err) => {
            eprintln!("hop impersonate: {err}");
            if matches!(err, ImpersonateError::Api(ApiError::Status(403))) {
                eprintln!(
                    "hint: you need roles/iam.serviceAccountTokenCreator on the target service account"
                );
            }
            err.exit_code()
        }
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("hop impersonate: {message}");
    ExitCode::FAILURE
}

// The testable body. Ok(None) means the user backed out (picker cancel or
// declined re-auth); nothing was written in that case.
fn impersonate_flow(
    ports: &Ports,
    context: &Context,
    settings: Settings,
    target: Option<&str>,
) -> Result<Option<ServiceAccount>, ImpersonateError> {
    let Some(account) = context.account.clone() else {
        return Err(ImpersonateError::NoAccount);
    };
    let token = match token_with_reauth(&ports.auth, settings, &account)? {
        TokenOutcome::Token(token) => token,
        TokenOutcome::Declined => return Ok(None),
    };
    let service_account = match target {
        Some(raw) => ServiceAccount::new(raw)?,
        None => {
            let Some(project) = context.project.clone() else {
                return Err(ImpersonateError::NoProject);
            };
            let accounts = ports.lister.list_service_accounts(&token, &project)?;
            if accounts.is_empty() {
                return Err(ImpersonateError::NoServiceAccounts(
                    project.as_str().to_string(),
                ));
            }
            match ports.picker.pick(&accounts)? {
                Some(choice) => choice,
                None => return Ok(None),
            }
        }
    };
    // Prove impersonation works before committing it to gcloud state, so a
    // missing role fails here with a clear error instead of breaking every
    // later gcloud call.
    ports
        .verifier
        .verify_impersonation(&token, &service_account)?;
    ports
        .store
        .set_impersonation(&context.name, Some(&service_account))?;
    Ok(Some(service_account))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::core::context::{Configuration, ServiceAccountInfo};
    use crate::core::ports::{Authenticator, Confirmer, TokenProvider};
    use crate::core::types::{AccessToken, AccountEmail, ProjectId};

    fn context(account: Option<&str>, project: Option<&str>) -> Context {
        Context {
            name: "work".to_string(),
            account: account.map(|a| AccountEmail::new(a).expect("valid")),
            project: project.map(|p| ProjectId::new(p).expect("valid")),
            impersonation: None,
            login_config_file: None,
        }
    }

    fn sa(email: &str) -> ServiceAccount {
        ServiceAccount::new(email).expect("valid")
    }

    /// Records the impersonation write; every other store method is
    /// unreachable in these flows and panics to prove it.
    struct FakeStore {
        impersonation_set: RefCell<Option<(String, Option<String>)>>,
    }

    impl FakeStore {
        fn new() -> Self {
            Self {
                impersonation_set: RefCell::new(None),
            }
        }
    }

    impl ConfigurationStore for FakeStore {
        fn list(&self) -> Result<Vec<Configuration>, ConfigError> {
            panic!("list must not be called");
        }

        fn activate(&self, _: &str) -> Result<(), ConfigError> {
            panic!("activate must not be called");
        }

        fn set_project(&self, _: &str, _: &ProjectId) -> Result<(), ConfigError> {
            panic!("set_project must not be called");
        }

        fn set_impersonation(
            &self,
            name: &str,
            service_account: Option<&ServiceAccount>,
        ) -> Result<(), ConfigError> {
            *self.impersonation_set.borrow_mut() = Some((
                name.to_string(),
                service_account.map(|sa| sa.as_str().to_string()),
            ));
            Ok(())
        }
    }

    struct FakeLister(Vec<ServiceAccountInfo>);

    impl ServiceAccountLister for FakeLister {
        fn list_service_accounts(
            &self,
            _: &AccessToken,
            _: &ProjectId,
        ) -> Result<Vec<ServiceAccountInfo>, ApiError> {
            Ok(self.0.clone())
        }
    }

    struct FakeVerifier {
        result: Result<(), u16>,
        called: RefCell<bool>,
    }

    impl FakeVerifier {
        fn ok() -> Self {
            Self {
                result: Ok(()),
                called: RefCell::new(false),
            }
        }

        fn denied() -> Self {
            Self {
                result: Err(403),
                called: RefCell::new(false),
            }
        }
    }

    impl ImpersonationVerifier for FakeVerifier {
        fn verify_impersonation(
            &self,
            _: &AccessToken,
            _: &ServiceAccount,
        ) -> Result<(), ApiError> {
            *self.called.borrow_mut() = true;
            self.result.map_err(ApiError::Status)
        }
    }

    struct FakePicker(Option<ServiceAccount>);

    impl ServiceAccountPicker for FakePicker {
        fn pick(&self, _: &[ServiceAccountInfo]) -> Result<Option<ServiceAccount>, PromptError> {
            Ok(self.0.clone())
        }
    }

    /// Valid tokens; login and confirm are unreachable and panic to prove it.
    struct ValidTokens;

    impl TokenProvider for ValidTokens {
        fn access_token(&self, _: &AccountEmail) -> Result<AccessToken, AuthError> {
            Ok(AccessToken::new("fake-token-for-tests").expect("valid"))
        }
    }

    impl Authenticator for ValidTokens {
        fn login(
            &self,
            _: Option<&AccountEmail>,
            _: bool,
            _: Option<&Path>,
        ) -> Result<(), AuthError> {
            panic!("login must not be called");
        }
    }

    impl Confirmer for ValidTokens {
        fn confirm(&self, _: &str) -> Result<Option<bool>, PromptError> {
            panic!("confirmer must not be called");
        }
    }

    struct Fixture {
        store: FakeStore,
        lister: FakeLister,
        verifier: FakeVerifier,
        picker: FakePicker,
        tokens: ValidTokens,
    }

    impl Fixture {
        fn ports(&self) -> Ports<'_> {
            Ports {
                store: &self.store,
                lister: &self.lister,
                verifier: &self.verifier,
                picker: &self.picker,
                auth: AuthFlow {
                    tokens: &self.tokens,
                    authenticator: &self.tokens,
                    confirmer: &self.tokens,
                    login_config: None,
                },
            }
        }
    }

    fn fixture() -> Fixture {
        Fixture {
            store: FakeStore::new(),
            lister: FakeLister(vec![ServiceAccountInfo {
                email: sa("deploy@my-project-123.iam.gserviceaccount.com"),
                display_name: Some("Deploy".to_string()),
            }]),
            verifier: FakeVerifier::ok(),
            picker: FakePicker(Some(sa("deploy@my-project-123.iam.gserviceaccount.com"))),
            tokens: ValidTokens,
        }
    }

    #[test]
    fn direct_target_is_verified_then_written() {
        // arrange
        let fix = fixture();
        // act
        let outcome = impersonate_flow(
            &fix.ports(),
            &context(Some("dev@example.com"), None),
            Settings::default(),
            Some("deploy@my-project-123.iam.gserviceaccount.com"),
        )
        .expect("flow failed");
        // assert
        assert!(outcome.is_some());
        assert!(*fix.verifier.called.borrow());
        assert_eq!(
            fix.store.impersonation_set.borrow().clone(),
            Some((
                "work".to_string(),
                Some("deploy@my-project-123.iam.gserviceaccount.com".to_string())
            ))
        );
    }

    #[test]
    fn a_denied_mint_writes_nothing_and_maps_to_permission_denied() {
        // arrange
        let mut fix = fixture();
        fix.verifier = FakeVerifier::denied();
        // act
        let err = impersonate_flow(
            &fix.ports(),
            &context(Some("dev@example.com"), None),
            Settings::default(),
            Some("deploy@my-project-123.iam.gserviceaccount.com"),
        )
        .expect_err("denied mint was accepted");
        // assert
        assert!(matches!(err, ImpersonateError::Api(ApiError::Status(403))));
        assert_eq!(fix.store.impersonation_set.borrow().clone(), None);
    }

    #[test]
    fn interactive_pick_verifies_and_writes() {
        // arrange
        let fix = fixture();
        // act
        let outcome = impersonate_flow(
            &fix.ports(),
            &context(Some("dev@example.com"), Some("my-project-123")),
            Settings::default(),
            None,
        )
        .expect("flow failed");
        // assert
        assert_eq!(
            outcome.map(|sa| sa.as_str().to_string()),
            Some("deploy@my-project-123.iam.gserviceaccount.com".to_string())
        );
        assert!(*fix.verifier.called.borrow());
    }

    #[test]
    fn cancelling_the_picker_writes_nothing() {
        // arrange
        let mut fix = fixture();
        fix.picker = FakePicker(None);
        // act
        let outcome = impersonate_flow(
            &fix.ports(),
            &context(Some("dev@example.com"), Some("my-project-123")),
            Settings::default(),
            None,
        )
        .expect("flow failed");
        // assert
        assert!(outcome.is_none());
        assert_eq!(fix.store.impersonation_set.borrow().clone(), None);
    }

    #[test]
    fn interactive_without_project_is_an_error() {
        // arrange
        let fix = fixture();
        // act
        let err = impersonate_flow(
            &fix.ports(),
            &context(Some("dev@example.com"), None),
            Settings::default(),
            None,
        )
        .expect_err("missing project was accepted");
        // assert
        assert!(matches!(err, ImpersonateError::NoProject));
    }

    #[test]
    fn no_account_is_an_error() {
        // arrange
        let fix = fixture();
        // act
        let err = impersonate_flow(
            &fix.ports(),
            &context(None, Some("my-project-123")),
            Settings::default(),
            None,
        )
        .expect_err("missing account was accepted");
        // assert
        assert!(matches!(err, ImpersonateError::NoAccount));
    }

    #[test]
    fn an_empty_service_account_list_is_an_error() {
        // arrange
        let mut fix = fixture();
        fix.lister = FakeLister(Vec::new());
        // act
        let err = impersonate_flow(
            &fix.ports(),
            &context(Some("dev@example.com"), Some("my-project-123")),
            Settings::default(),
            None,
        )
        .expect_err("empty list was accepted");
        // assert
        assert!(matches!(err, ImpersonateError::NoServiceAccounts(_)));
    }

    #[test]
    fn an_invalid_direct_target_is_bad_input() {
        // arrange
        let fix = fixture();
        // act
        let err = impersonate_flow(
            &fix.ports(),
            &context(Some("dev@example.com"), None),
            Settings::default(),
            Some("not a service account"),
        )
        .expect_err("invalid service account was accepted");
        // assert
        assert!(matches!(err, ImpersonateError::InvalidServiceAccount(_)));
    }
}
