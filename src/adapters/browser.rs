//! Opening URLs in the system browser via the `open` crate (cross-platform;
//! no shelling out to platform tools by hand, rules/cross-platform.md).

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
