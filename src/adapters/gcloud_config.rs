//! Read/write access to gcloud's configuration state on disk.
//! Formats are gcloud's own and unstable: parse defensively
//! (rules/gcloud-safety.md). Writing is limited to hop's core action of
//! switching the active configuration.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use crate::adapters::gcloud_ini::GcloudIni;
use crate::core::context::{Configuration, Context};
use crate::core::error::{ConfigError, ValidationError};
use crate::core::ports::{ConfigurationStore, ContextSource};
use crate::core::types::{AccountEmail, ProjectId, ServiceAccount};

/// Environment variable gcloud itself honors to relocate its config directory.
pub const CLOUDSDK_CONFIG_ENV: &str = "CLOUDSDK_CONFIG";

/// Resolve the gcloud configuration directory for this platform.
/// The single source of truth for this path; nothing else may construct it.
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    resolve_config_dir(env::var_os(CLOUDSDK_CONFIG_ENV))
}

fn resolve_config_dir(override_var: Option<OsString>) -> Result<PathBuf, ConfigError> {
    match override_var {
        Some(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => platform_default_dir(),
    }
}

// gcloud uses %APPDATA%\gcloud on Windows, not a home-relative path.
#[cfg(windows)]
fn platform_default_dir() -> Result<PathBuf, ConfigError> {
    env::var_os("APPDATA")
        .filter(|appdata| !appdata.is_empty())
        .map(|appdata| PathBuf::from(appdata).join("gcloud"))
        .ok_or(ConfigError::HomeDirUnavailable)
}

// gcloud uses ~/.config/gcloud on both Linux and macOS (it does not follow
// the macOS ~/Library convention).
#[cfg(not(windows))]
fn platform_default_dir() -> Result<PathBuf, ConfigError> {
    env::home_dir()
        .map(|home| home.join(".config").join("gcloud"))
        .ok_or(ConfigError::HomeDirUnavailable)
}

/// Reads and writes the active context via gcloud's config files on disk.
///
/// The production implementation of [`ContextSource`] and
/// [`ConfigurationStore`]; the directory is resolved once at construction so
/// every access agrees on the location.
pub struct GcloudConfigSource {
    config_dir: PathBuf,
}

impl GcloudConfigSource {
    /// Build a source rooted at the resolved gcloud config directory.
    pub fn new() -> Result<Self, ConfigError> {
        Ok(Self {
            config_dir: config_dir()?,
        })
    }

    /// The directory this source reads from.
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

impl ContextSource for GcloudConfigSource {
    fn active_context(&self) -> Result<Context, ConfigError> {
        let name = active_config_name(&self.config_dir)?;
        let path = configuration_file(&self.config_dir, &name);
        let ini = load_configuration(&path)?;
        // Closures rather than bare `T::new` paths: the constructors are
        // generic over `impl Into<String>`, which cannot coerce to the
        // higher-ranked `for<'a> Fn(&'a str)` that `property` needs.
        Ok(Context {
            account: property(&ini, &path, "core", "account", |raw| AccountEmail::new(raw))?,
            project: property(&ini, &path, "core", "project", |raw| ProjectId::new(raw))?,
            impersonation: property(&ini, &path, "auth", "impersonate_service_account", |raw| {
                ServiceAccount::new(raw)
            })?,
            name,
        })
    }
}

impl ConfigurationStore for GcloudConfigSource {
    fn list(&self) -> Result<Vec<Configuration>, ConfigError> {
        let dir = self.config_dir.join("configurations");
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            // No directory yet simply means gcloud has never been set up.
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(ConfigError::Unreadable { path: dir, source }),
        };
        let active = active_config_name(&self.config_dir)?;
        let mut configurations = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| ConfigError::Unreadable {
                path: dir.clone(),
                source,
            })?;
            // Only `config_<name>` files count; gcloud keeps nothing else here.
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str().and_then(|n| n.strip_prefix("config_")) else {
                continue;
            };
            if name.is_empty() || !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let path = entry.path();
            let ini = load_configuration(&path)?;
            configurations.push(Configuration {
                name: name.to_string(),
                account: property(&ini, &path, "core", "account", |raw| AccountEmail::new(raw))?,
                project: property(&ini, &path, "core", "project", |raw| ProjectId::new(raw))?,
                is_active: name == active,
            });
        }
        configurations.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(configurations)
    }

    fn activate(&self, name: &str) -> Result<(), ConfigError> {
        if !configuration_file(&self.config_dir, name).is_file() {
            return Err(ConfigError::UnknownConfiguration {
                name: name.to_string(),
            });
        }
        let path = self.config_dir.join("active_config");
        // Write-then-rename: active_config can never be left truncated or
        // half-written, which is the safety net the plan asks for.
        let temp = self.config_dir.join("active_config.hop-tmp");
        fs::write(&temp, name).map_err(|source| ConfigError::WriteFailed {
            path: temp.clone(),
            source,
        })?;
        fs::rename(&temp, &path).map_err(|source| {
            // Best effort: a stray temp file is harmless but untidy.
            let _ = fs::remove_file(&temp);
            ConfigError::WriteFailed { path, source }
        })
    }
}

/// Name of the currently active gcloud configuration.
///
/// gcloud stores it as the plain-text content of `active_config` and falls
/// back to "default" when the file is absent; so do we.
pub fn active_config_name(config_dir: &Path) -> Result<String, ConfigError> {
    let path = config_dir.join("active_config");
    match fs::read_to_string(&path) {
        Ok(name) => Ok(name.trim().to_string()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok("default".to_string()),
        Err(source) => Err(ConfigError::Unreadable { path, source }),
    }
}

fn configuration_file(config_dir: &Path, name: &str) -> PathBuf {
    config_dir
        .join("configurations")
        .join(format!("config_{name}"))
}

// A named configuration can exist without a file (gcloud treats that as an
// empty configuration), so absence parses as empty rather than failing.
fn load_configuration(path: &Path) -> Result<GcloudIni, ConfigError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(source) => {
            return Err(ConfigError::Unreadable {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    GcloudIni::parse(&text).map_err(|err| ConfigError::Malformed {
        path: path.to_path_buf(),
        detail: err.to_string(),
    })
}

// Read one optional property and lift it into its validated newtype.
fn property<T>(
    ini: &GcloudIni,
    path: &Path,
    section: &str,
    key: &str,
    make: impl Fn(&str) -> Result<T, ValidationError>,
) -> Result<Option<T>, ConfigError> {
    ini.get(section, key)
        .map(&make)
        .transpose()
        .map_err(|source| ConfigError::InvalidProperty {
            path: path.to_path_buf(),
            property: format!("{section}/{key}"),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique per-test scratch directory; std-only stand-in for tempfile.
    fn scratch_dir(test: &str) -> PathBuf {
        let dir = env::temp_dir()
            .join("hop-tests")
            .join(format!("{test}-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("failed to create scratch dir");
        dir
    }

    fn write_configuration(config_dir: &Path, name: &str, contents: &str) {
        let dir = config_dir.join("configurations");
        fs::create_dir_all(&dir).expect("failed to create configurations dir");
        fs::write(dir.join(format!("config_{name}")), contents).expect("write failed");
    }

    #[test]
    fn override_env_wins_over_platform_default() {
        // arrange
        let override_var = Some(OsString::from("/custom/gcloud-config"));
        // act
        let dir = resolve_config_dir(override_var).expect("resolution failed");
        // assert
        assert_eq!(dir, PathBuf::from("/custom/gcloud-config"));
    }

    #[test]
    fn empty_override_falls_back_to_platform_default() {
        // arrange
        let override_var = Some(OsString::new());
        // act
        let dir = resolve_config_dir(override_var).expect("resolution failed");
        // assert
        assert!(dir.ends_with("gcloud"), "unexpected dir: {}", dir.display());
    }

    #[test]
    fn active_config_name_reads_and_trims_the_file() {
        // arrange
        let dir = scratch_dir("active-config-present");
        fs::write(dir.join("active_config"), "work\n").expect("write failed");
        // act
        let name = active_config_name(&dir).expect("read failed");
        // assert
        assert_eq!(name, "work");
    }

    #[test]
    fn active_config_name_defaults_when_file_is_absent() {
        // arrange
        let dir = scratch_dir("active-config-absent");
        // act
        let name = active_config_name(&dir).expect("read failed");
        // assert
        assert_eq!(name, "default");
    }

    #[test]
    fn active_context_reads_all_properties() {
        // arrange
        let dir = scratch_dir("context-full");
        fs::write(dir.join("active_config"), "work").expect("write failed");
        write_configuration(
            &dir,
            "work",
            "[core]\naccount = dev@example.com\nproject = my-project-123\n\n[auth]\nimpersonate_service_account = sa@my-project-123.iam.gserviceaccount.com\n",
        );
        let source = GcloudConfigSource { config_dir: dir };
        // act
        let context = source.active_context().expect("read failed");
        // assert
        assert_eq!(context.name, "work");
        assert_eq!(
            context.account,
            Some(AccountEmail::new("dev@example.com").expect("valid"))
        );
        assert_eq!(
            context.project,
            Some(ProjectId::new("my-project-123").expect("valid"))
        );
        assert_eq!(
            context.impersonation,
            Some(ServiceAccount::new("sa@my-project-123.iam.gserviceaccount.com").expect("valid"))
        );
    }

    #[test]
    fn active_context_with_missing_file_is_bare() {
        // arrange: active_config names a configuration that has no file
        let dir = scratch_dir("context-missing-file");
        fs::write(dir.join("active_config"), "ghost").expect("write failed");
        let source = GcloudConfigSource { config_dir: dir };
        // act
        let context = source.active_context().expect("read failed");
        // assert
        assert_eq!(context.name, "ghost");
        assert_eq!(context.account, None);
        assert_eq!(context.project, None);
        assert_eq!(context.impersonation, None);
    }

    #[test]
    fn active_context_rejects_an_invalid_account() {
        // arrange
        let dir = scratch_dir("context-bad-account");
        write_configuration(&dir, "default", "[core]\naccount = not an email\n");
        let source = GcloudConfigSource { config_dir: dir };
        // act
        let err = source.active_context().expect_err("accepted bad account");
        // assert
        assert!(
            matches!(&err, ConfigError::InvalidProperty { property, .. } if property == "core/account"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn active_context_reports_a_malformed_file() {
        // arrange
        let dir = scratch_dir("context-malformed");
        write_configuration(&dir, "default", "not an ini file\n");
        let source = GcloudConfigSource { config_dir: dir };
        // act
        let err = source
            .active_context()
            .expect_err("accepted malformed file");
        // assert
        assert!(
            matches!(err, ConfigError::Malformed { .. }),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn list_returns_sorted_configurations_with_active_flag() {
        // arrange
        let dir = scratch_dir("list-sorted");
        fs::write(dir.join("active_config"), "work").expect("write failed");
        write_configuration(&dir, "work", "[core]\naccount = dev@example.com\n");
        write_configuration(
            &dir,
            "default",
            "[core]\naccount = dev@example.com\nproject = my-project-123\n",
        );
        let source = GcloudConfigSource { config_dir: dir };
        // act
        let configurations = source.list().expect("list failed");
        // assert
        assert_eq!(configurations.len(), 2);
        assert_eq!(configurations[0].name, "default");
        assert!(!configurations[0].is_active);
        assert_eq!(
            configurations[0].project,
            Some(ProjectId::new("my-project-123").expect("valid"))
        );
        assert_eq!(configurations[1].name, "work");
        assert!(configurations[1].is_active);
        assert_eq!(configurations[1].project, None);
    }

    #[test]
    fn list_is_empty_without_a_configurations_dir() {
        // arrange
        let dir = scratch_dir("list-empty");
        let source = GcloudConfigSource { config_dir: dir };
        // act
        let configurations = source.list().expect("list failed");
        // assert
        assert!(configurations.is_empty());
    }

    #[test]
    fn activate_switches_the_active_configuration() {
        // arrange
        let dir = scratch_dir("activate-ok");
        write_configuration(&dir, "default", "");
        write_configuration(&dir, "work", "[core]\naccount = dev@example.com\n");
        fs::write(dir.join("active_config"), "default").expect("write failed");
        let source = GcloudConfigSource {
            config_dir: dir.clone(),
        };
        // act
        source.activate("work").expect("activate failed");
        // assert
        let written = fs::read_to_string(dir.join("active_config")).expect("read failed");
        assert_eq!(written, "work");
        assert!(
            !dir.join("active_config.hop-tmp").exists(),
            "temp file left behind"
        );
    }

    #[test]
    fn activate_rejects_an_unknown_configuration() {
        // arrange
        let dir = scratch_dir("activate-unknown");
        write_configuration(&dir, "default", "");
        fs::write(dir.join("active_config"), "default").expect("write failed");
        let source = GcloudConfigSource {
            config_dir: dir.clone(),
        };
        // act
        let err = source.activate("nope").expect_err("activated a ghost");
        // assert
        assert!(
            matches!(&err, ConfigError::UnknownConfiguration { name } if name == "nope"),
            "unexpected error: {err}"
        );
        let untouched = fs::read_to_string(dir.join("active_config")).expect("read failed");
        assert_eq!(untouched, "default", "active_config must be untouched");
    }

    #[test]
    fn source_reads_the_active_context_through_the_port() {
        // arrange
        let dir = scratch_dir("source-active-context");
        fs::write(dir.join("active_config"), "work\n").expect("write failed");
        let source = GcloudConfigSource { config_dir: dir };
        // act: call through the trait, exactly as commands will
        let context = ContextSource::active_context(&source).expect("read failed");
        // assert
        assert_eq!(context.name, "work");
        assert_eq!(context.account, None);
        assert_eq!(context.project, None);
        assert_eq!(context.impersonation, None);
    }
}
