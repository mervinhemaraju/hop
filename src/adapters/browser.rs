//! Opening URLs in the system browser via the `open` crate (cross-platform;
//! no shelling out to platform tools by hand, rules/cross-platform.md).

use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::core::error::BrowserError;
use crate::core::ports::BrowserOpener;

/// The operating system's default URL handler.
pub struct SystemBrowser;

impl BrowserOpener for SystemBrowser {
    fn open_url(&self, url: &str) -> Result<(), BrowserError> {
        open::that(url).map_err(|err| BrowserError {
            detail: err.to_string(),
        })
    }
}

/// A user-chosen browser command (the `browser` setting or the BROWSER
/// environment variable), invoked with the URL as its single argument.
pub struct CustomBrowser {
    command: PathBuf,
}

impl CustomBrowser {
    /// Open URLs by running `command <url>`.
    pub fn new(command: PathBuf) -> Self {
        Self { command }
    }
}

impl BrowserOpener for CustomBrowser {
    fn open_url(&self, url: &str) -> Result<(), BrowserError> {
        // Spawn without waiting: the command may stay in the foreground
        // until the browser exits, and hop must not hang on that. stdout is
        // nulled so the command can never pollute hop's machine-readable
        // stream (rules/cli-ux.md); its stderr stays visible for diagnostics.
        Command::new(&self.command)
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .map(drop)
            .map_err(|err| BrowserError {
                detail: format!(
                    "browser command {} did not start: {err}",
                    self.command.display()
                ),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_browser_reports_a_missing_command() {
        // arrange
        let browser = CustomBrowser::new(PathBuf::from("/nonexistent/hop-test-browser"));
        // act
        let err = browser
            .open_url("https://example.com")
            .expect_err("missing command was accepted");
        // assert
        assert!(err.detail.contains("hop-test-browser"));
    }
}
