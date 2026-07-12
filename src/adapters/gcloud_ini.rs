//! Minimal parser and editor for gcloud's configuration files.
//!
//! gcloud writes these with Python's configparser: `[section]` headers,
//! `key = value` properties (configparser also accepts `:`), `#`/`;`
//! comments, and lowercased keys. The format is gcloud's own and unstable,
//! so this parses the subset gcloud actually emits and fails loudly on
//! anything else (rules/gcloud-safety.md).

use std::collections::HashMap;

use thiserror::Error;

/// Failures while parsing a gcloud configuration file. Line numbers are
/// 1-based so they can be quoted to the user directly.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IniParseError {
    #[error("line {line}: property {key:?} appears before any [section] header")]
    PropertyOutsideSection { line: usize, key: String },
    #[error("line {line}: empty section header")]
    EmptySectionName { line: usize },
    #[error(
        "line {line}: indented continuation lines are not supported; put each property on a single line"
    )]
    IndentedLine { line: usize },
    #[error("line {line}: expected `[section]` or `key = value`, found {content:?}")]
    UnrecognizedLine { line: usize, content: String },
}

/// A parsed gcloud configuration file: section name to key/value map.
#[derive(Debug)]
pub struct GcloudIni {
    sections: HashMap<String, HashMap<String, String>>,
}

impl GcloudIni {
    /// Parse a configuration file's text.
    pub fn parse(text: &str) -> Result<Self, IniParseError> {
        // Tolerate a UTF-8 BOM; some Windows editors add one.
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut current: Option<String> = None;

        for (index, raw) in text.lines().enumerate() {
            let number = index + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            // configparser treats indented lines as value continuations;
            // gcloud does not emit them, so refuse rather than misparse.
            if raw.starts_with([' ', '\t']) {
                return Err(IniParseError::IndentedLine { line: number });
            }
            if let Some(inner) = line
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
            {
                let name = inner.trim();
                if name.is_empty() {
                    return Err(IniParseError::EmptySectionName { line: number });
                }
                sections.entry(name.to_string()).or_default();
                current = Some(name.to_string());
                continue;
            }
            let Some((key, value)) = split_property_line(line) else {
                return Err(IniParseError::UnrecognizedLine {
                    line: number,
                    content: line.to_string(),
                });
            };
            let Some(section) = current.clone() else {
                return Err(IniParseError::PropertyOutsideSection { line: number, key });
            };
            // Last occurrence wins, mirroring a forgiving read of duplicates.
            sections.entry(section).or_default().insert(key, value);
        }
        Ok(Self { sections })
    }

    /// Look up a property. An empty value counts as unset, mirroring what
    /// `gcloud config unset` leaves behind in older gcloud versions.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        let value = self.sections.get(section)?.get(&key.to_ascii_lowercase())?;
        if value.is_empty() { None } else { Some(value) }
    }
}

// Split `key = value` (or `key: value`), lowercasing the key the way
// configparser's default optionxform does.
fn split_property_line(line: &str) -> Option<(String, String)> {
    let position = line.find(['=', ':'])?;
    let key = line[..position].trim();
    if key.is_empty() {
        return None;
    }
    let value = line[position + 1..].trim();
    Some((key.to_ascii_lowercase(), value.to_string()))
}

/// Set `section`/`key` to `value` in a configuration file's text, touching
/// nothing else: every other line is preserved byte for byte, including
/// comments, spacing, and CRLF line endings.
// Dead-code allowance: the consumers are the project-property write (Phase 3)
// and impersonation (Phase 4). Tests pin the behaviour until then.
#[allow(dead_code)]
pub fn set_property(text: &str, section: &str, key: &str, value: &str) -> String {
    let key = key.to_ascii_lowercase();
    let mut output =
        String::with_capacity(text.len() + section.len() + key.len() + value.len() + 8);
    let mut in_target = false;
    let mut section_seen = false;
    let mut done = false;

    for raw in text.split_inclusive('\n') {
        let (content, terminator) = split_line_terminator(raw);
        let trimmed = content.trim();
        let is_header = trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2;
        if is_header {
            // Leaving the target section without a hit: insert before the
            // next header so the property lands inside its section.
            if in_target && !done {
                output.push_str(&format!("{key} = {value}\n"));
                done = true;
            }
            let name = trimmed[1..trimmed.len() - 1].trim();
            in_target = name == section;
            section_seen = section_seen || in_target;
        } else if in_target && !done {
            if let Some((existing_key, _)) = split_property_line(trimmed) {
                if existing_key == key {
                    output.push_str(&format!("{key} = {value}"));
                    output.push_str(terminator);
                    done = true;
                    continue;
                }
            }
        }
        output.push_str(raw);
    }

    if !done {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        if !section_seen {
            output.push_str(&format!("[{section}]\n"));
        }
        output.push_str(&format!("{key} = {value}\n"));
    }
    output
}

#[allow(dead_code)]
fn split_line_terminator(raw: &str) -> (&str, &str) {
    if let Some(content) = raw.strip_suffix("\r\n") {
        (content, "\r\n")
    } else if let Some(content) = raw.strip_suffix('\n') {
        (content, "\n")
    } else {
        (raw, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPICAL: &str = "\
[core]
account = dev@example.com
project = my-project-123

[auth]
impersonate_service_account = sa@my-project-123.iam.gserviceaccount.com
";

    #[test]
    fn parses_a_typical_gcloud_configuration() {
        // act
        let ini = GcloudIni::parse(TYPICAL).expect("parse failed");
        // assert
        assert_eq!(ini.get("core", "account"), Some("dev@example.com"));
        assert_eq!(ini.get("core", "project"), Some("my-project-123"));
        assert_eq!(
            ini.get("auth", "impersonate_service_account"),
            Some("sa@my-project-123.iam.gserviceaccount.com")
        );
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        // arrange
        let text = "# a comment\n\n[core]\n; another comment\nproject = my-project-123\n";
        // act
        let ini = GcloudIni::parse(text).expect("parse failed");
        // assert
        assert_eq!(ini.get("core", "project"), Some("my-project-123"));
    }

    #[test]
    fn accepts_colon_as_delimiter_and_lowercases_keys() {
        // arrange
        let text = "[core]\nProject: my-project-123\n";
        // act
        let ini = GcloudIni::parse(text).expect("parse failed");
        // assert
        assert_eq!(ini.get("core", "project"), Some("my-project-123"));
    }

    #[test]
    fn treats_an_empty_value_as_unset() {
        // arrange
        let text = "[core]\nproject =\n";
        // act
        let ini = GcloudIni::parse(text).expect("parse failed");
        // assert
        assert_eq!(ini.get("core", "project"), None);
    }

    #[test]
    fn missing_section_or_key_is_none() {
        // arrange
        let ini = GcloudIni::parse(TYPICAL).expect("parse failed");
        // assert
        assert_eq!(ini.get("compute", "zone"), None);
        assert_eq!(ini.get("core", "zone"), None);
    }

    #[test]
    fn last_duplicate_key_wins() {
        // arrange
        let text = "[core]\nproject = old-project\nproject = new-project\n";
        // act
        let ini = GcloudIni::parse(text).expect("parse failed");
        // assert
        assert_eq!(ini.get("core", "project"), Some("new-project"));
    }

    #[test]
    fn strips_a_leading_bom() {
        // arrange
        let text = "\u{feff}[core]\nproject = my-project-123\n";
        // act
        let ini = GcloudIni::parse(text).expect("parse failed");
        // assert
        assert_eq!(ini.get("core", "project"), Some("my-project-123"));
    }

    #[test]
    fn rejects_a_property_before_any_section() {
        // act
        let err = GcloudIni::parse("project = my-project-123\n").expect_err("accepted");
        // assert
        assert_eq!(
            err,
            IniParseError::PropertyOutsideSection {
                line: 1,
                key: "project".to_string(),
            }
        );
    }

    #[test]
    fn rejects_an_unrecognized_line() {
        // act
        let err = GcloudIni::parse("[core]\nwhat is this\n").expect_err("accepted");
        // assert
        assert_eq!(
            err,
            IniParseError::UnrecognizedLine {
                line: 2,
                content: "what is this".to_string(),
            }
        );
    }

    #[test]
    fn rejects_indented_continuation_lines() {
        // act
        let err = GcloudIni::parse("[core]\nproject = a\n  continued\n").expect_err("accepted");
        // assert
        assert_eq!(err, IniParseError::IndentedLine { line: 3 });
    }

    #[test]
    fn rejects_an_empty_section_header() {
        // act
        let err = GcloudIni::parse("[ ]\n").expect_err("accepted");
        // assert
        assert_eq!(err, IniParseError::EmptySectionName { line: 1 });
    }

    #[test]
    fn set_property_replaces_a_value_and_preserves_everything_else() {
        // arrange
        let text = "# managed by gcloud\n[core]\naccount =  dev@example.com \nproject = old-project\n\n[auth]\nx = y\n";
        // act
        let updated = set_property(text, "core", "project", "new-project");
        // assert: untouched lines are byte-identical, including odd spacing
        assert_eq!(
            updated,
            "# managed by gcloud\n[core]\naccount =  dev@example.com \nproject = new-project\n\n[auth]\nx = y\n"
        );
    }

    #[test]
    fn set_property_adds_a_key_to_an_existing_section() {
        // arrange
        let text = "[core]\naccount = dev@example.com\n[auth]\nx = y\n";
        // act
        let updated = set_property(text, "core", "project", "my-project-123");
        // assert
        assert_eq!(
            updated,
            "[core]\naccount = dev@example.com\nproject = my-project-123\n[auth]\nx = y\n"
        );
    }

    #[test]
    fn set_property_appends_a_missing_section() {
        // arrange
        let text = "[core]\naccount = dev@example.com\n";
        // act
        let updated = set_property(
            text,
            "auth",
            "impersonate_service_account",
            "sa@my-project-123.iam.gserviceaccount.com",
        );
        // assert
        assert_eq!(
            updated,
            "[core]\naccount = dev@example.com\n[auth]\nimpersonate_service_account = sa@my-project-123.iam.gserviceaccount.com\n"
        );
    }

    #[test]
    fn set_property_starts_from_empty_text() {
        // act
        let updated = set_property("", "core", "project", "my-project-123");
        // assert
        assert_eq!(updated, "[core]\nproject = my-project-123\n");
    }

    #[test]
    fn set_property_preserves_crlf_on_untouched_lines() {
        // arrange
        let text = "[core]\r\naccount = dev@example.com\r\nproject = old\r\n";
        // act
        let updated = set_property(text, "core", "project", "new");
        // assert: the replaced line keeps its own CRLF terminator too
        assert_eq!(
            updated,
            "[core]\r\naccount = dev@example.com\r\nproject = new\r\n"
        );
    }

    #[test]
    fn set_property_matches_keys_case_insensitively() {
        // arrange
        let text = "[core]\nProject = old\n";
        // act
        let updated = set_property(text, "core", "project", "new");
        // assert
        assert_eq!(updated, "[core]\nproject = new\n");
    }
}
