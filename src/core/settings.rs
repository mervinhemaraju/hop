//! hop's own user settings (not gcloud state).

/// What to do when a switch needs credentials that turn out to be expired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReauthPolicy {
    /// Ask before launching the login flow (the granted.dev-style default).
    #[default]
    Prompt,
    /// Launch the login flow immediately without asking.
    Auto,
    /// Never launch the login flow; fail and let the user run `hop login`.
    Off,
}

/// All of hop's settings, with defaults for anything unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Settings {
    /// Re-authentication behaviour on expired credentials.
    pub reauth: ReauthPolicy,
}
