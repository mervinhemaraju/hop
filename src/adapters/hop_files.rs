//! hop's own settings and cache files, kept apart from gcloud's tree.
//! Settings live at `settings.json`; project caches at
//! `cache/projects-<account>.json`. Cache files carry only project ids and
//! display names (no credentials) but are still written `0600` on Unix.

use std::path::{Path, PathBuf};
use std::{env, fs, io};

use serde::{Deserialize, Serialize};

use crate::core::context::Project;
use crate::core::error::ConfigError;
use crate::core::ports::{ProjectCache, SettingsStore};
use crate::core::settings::{ReauthPolicy, Settings};
use crate::core::types::{AccountEmail, ProjectId};

/// Environment variable overriding hop's own config directory (mirrors the
/// CLOUDSDK_CONFIG pattern; used by tests and end-to-end runs).
pub const HOP_CONFIG_ENV: &str = "HOP_CONFIG";

/// Settings and caches rooted in hop's config directory.
pub struct HopFiles {
    dir: PathBuf,
}

impl HopFiles {
    /// Root at the resolved hop config directory.
    pub fn new() -> Result<Self, ConfigError> {
        Ok(Self { dir: hop_dir()? })
    }
}

fn hop_dir() -> Result<PathBuf, ConfigError> {
    match env::var_os(HOP_CONFIG_ENV) {
        Some(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => platform_default_dir(),
    }
}

// Same per-platform convention as the gcloud dir resolver: %APPDATA% on
// Windows, ~/.config elsewhere.
#[cfg(windows)]
fn platform_default_dir() -> Result<PathBuf, ConfigError> {
    env::var_os("APPDATA")
        .filter(|appdata| !appdata.is_empty())
        .map(|appdata| PathBuf::from(appdata).join("hop"))
        .ok_or(ConfigError::HomeDirUnavailable)
}

#[cfg(not(windows))]
fn platform_default_dir() -> Result<PathBuf, ConfigError> {
    env::home_dir()
        .map(|home| home.join(".config").join("hop"))
        .ok_or(ConfigError::HomeDirUnavailable)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SettingsFile {
    reauth: Option<String>,
}

impl SettingsStore for HopFiles {
    fn settings(&self) -> Result<Settings, ConfigError> {
        let path = self.dir.join("settings.json");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(Settings::default());
            }
            Err(source) => return Err(ConfigError::Unreadable { path, source }),
        };
        let file: SettingsFile =
            serde_json::from_str(&text).map_err(|err| ConfigError::Malformed {
                path: path.clone(),
                detail: err.to_string(),
            })?;
        let reauth = match file.reauth.as_deref() {
            None => ReauthPolicy::default(),
            Some("prompt") => ReauthPolicy::Prompt,
            Some("auto") => ReauthPolicy::Auto,
            Some("off") => ReauthPolicy::Off,
            Some(other) => {
                return Err(ConfigError::Malformed {
                    path,
                    detail: format!(
                        "unknown reauth value {other:?}; use \"prompt\", \"auto\", or \"off\""
                    ),
                });
            }
        };
        Ok(Settings { reauth })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheFile {
    projects: Vec<CachedProject>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedProject {
    project_id: String,
    display_name: Option<String>,
}

impl ProjectCache for HopFiles {
    fn cached_projects(&self, account: &AccountEmail) -> Result<Option<Vec<Project>>, ConfigError> {
        let path = self.cache_path(account);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(ConfigError::Unreadable { path, source }),
        };
        let file: CacheFile =
            serde_json::from_str(&text).map_err(|err| ConfigError::Malformed {
                path: path.clone(),
                detail: err.to_string(),
            })?;
        let projects = file
            .projects
            .into_iter()
            .map(|cached| {
                Ok(Project {
                    id: ProjectId::new(cached.project_id).map_err(|err| {
                        ConfigError::Malformed {
                            path: path.clone(),
                            detail: err.to_string(),
                        }
                    })?,
                    display_name: cached.display_name,
                })
            })
            .collect::<Result<Vec<_>, ConfigError>>()?;
        Ok(Some(projects))
    }

    fn store_projects(
        &self,
        account: &AccountEmail,
        projects: &[Project],
    ) -> Result<(), ConfigError> {
        let path = self.cache_path(account);
        let parent = path.parent().unwrap_or(&self.dir).to_path_buf();
        fs::create_dir_all(&parent).map_err(|source| ConfigError::WriteFailed {
            path: parent.clone(),
            source,
        })?;
        let file = CacheFile {
            projects: projects
                .iter()
                .map(|project| CachedProject {
                    project_id: project.id.as_str().to_string(),
                    display_name: project.display_name.clone(),
                })
                .collect(),
        };
        // Serializing our own plain struct cannot fail; a failure here is a
        // hop bug, so an expect with that meaning is acceptable.
        let text = serde_json::to_string_pretty(&file).expect("cache serialization is infallible");
        write_restricted(&path, &text)
    }
}

impl HopFiles {
    fn cache_path(&self, account: &AccountEmail) -> PathBuf {
        // Account emails are validated visible-ASCII, but path separators
        // and drive colons must still never reach the filesystem layer.
        let safe: String = account
            .as_str()
            .chars()
            .map(|c| {
                if matches!(c, '/' | '\\' | ':') {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        self.dir.join("cache").join(format!("projects-{safe}.json"))
    }
}

// Write via temp + rename with owner-only permissions on the temp file, so
// the final file is never world-readable at any point.
fn write_restricted(path: &Path, contents: &str) -> Result<(), ConfigError> {
    let temp = path.with_extension("hop-tmp");
    fs::write(&temp, contents).map_err(|source| ConfigError::WriteFailed {
        path: temp.clone(),
        source,
    })?;
    restrict_permissions(&temp)?;
    fs::rename(&temp, path).map_err(|source| {
        let _ = fs::remove_file(&temp);
        ConfigError::WriteFailed {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        ConfigError::WriteFailed {
            path: path.to_path_buf(),
            source,
        }
    })
}

// Windows: %APPDATA% is already per-user (protected by the profile ACL),
// which is the closest equivalent to 0600 without pulling in winapi ACL
// manipulation for a file that holds no credentials.
#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), ConfigError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(test: &str) -> PathBuf {
        let dir = env::temp_dir()
            .join("hop-tests")
            .join(format!("{test}-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("failed to create scratch dir");
        dir
    }

    fn account() -> AccountEmail {
        AccountEmail::new("dev@example.com").expect("valid")
    }

    fn sample_projects() -> Vec<Project> {
        vec![Project {
            id: ProjectId::new("my-project-123").expect("valid"),
            display_name: Some("My Project".to_string()),
        }]
    }

    #[test]
    fn settings_default_when_file_is_absent() {
        // arrange
        let files = HopFiles {
            dir: scratch_dir("settings-absent"),
        };
        // act
        let settings = files.settings().expect("load failed");
        // assert
        assert_eq!(settings.reauth, ReauthPolicy::Prompt);
    }

    #[test]
    fn settings_parse_each_reauth_value() {
        // arrange
        let dir = scratch_dir("settings-values");
        let files = HopFiles { dir: dir.clone() };
        for (raw, expected) in [
            ("prompt", ReauthPolicy::Prompt),
            ("auto", ReauthPolicy::Auto),
            ("off", ReauthPolicy::Off),
        ] {
            fs::write(
                dir.join("settings.json"),
                format!("{{\"reauth\": \"{raw}\"}}"),
            )
            .expect("write failed");
            // act
            let settings = files.settings().expect("load failed");
            // assert
            assert_eq!(settings.reauth, expected, "for value {raw}");
        }
    }

    #[test]
    fn settings_reject_an_unknown_reauth_value() {
        // arrange
        let dir = scratch_dir("settings-bad");
        fs::write(dir.join("settings.json"), "{\"reauth\": \"sometimes\"}").expect("write failed");
        let files = HopFiles { dir };
        // act
        let err = files.settings().expect_err("accepted bad value");
        // assert
        assert!(matches!(err, ConfigError::Malformed { .. }));
    }

    #[test]
    fn cache_round_trips_projects() {
        // arrange
        let files = HopFiles {
            dir: scratch_dir("cache-roundtrip"),
        };
        let projects = sample_projects();
        // act
        files
            .store_projects(&account(), &projects)
            .expect("store failed");
        let loaded = files.cached_projects(&account()).expect("load failed");
        // assert
        assert_eq!(loaded, Some(projects));
    }

    #[test]
    fn cache_misses_return_none() {
        // arrange
        let files = HopFiles {
            dir: scratch_dir("cache-miss"),
        };
        // act
        let loaded = files.cached_projects(&account()).expect("load failed");
        // assert
        assert_eq!(loaded, None);
    }

    #[cfg(unix)]
    #[test]
    fn cache_files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        // arrange
        let files = HopFiles {
            dir: scratch_dir("cache-perms"),
        };
        // act
        files
            .store_projects(&account(), &sample_projects())
            .expect("store failed");
        // assert
        let path = files.cache_path(&account());
        let mode = fs::metadata(path)
            .expect("stat failed")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "unexpected mode {mode:o}");
    }
}
