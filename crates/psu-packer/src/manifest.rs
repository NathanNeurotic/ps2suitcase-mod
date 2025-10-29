use std::fs;
use std::path::Path;

use camino::Utf8PathBuf;
use chrono::{DateTime, NaiveDateTime, Timelike, Utc};
use serde::Deserialize;
use thiserror::Error;

use crate::{error::PsuPackError, fixed_timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
    pub help: Option<String>,
    pub severity: IssueSeverity,
}

impl ValidationIssue {
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            help: None,
            severity: IssueSeverity::Error,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampPolicy {
    Fixed,
    Source,
    Now,
}

impl TimestampPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimestampPolicy::Fixed => "fixed",
            TimestampPolicy::Source => "source",
            TimestampPolicy::Now => "now",
        }
    }
}

impl Default for TimestampPolicy {
    fn default() -> Self {
        TimestampPolicy::Fixed
    }
}

impl std::fmt::Display for TimestampPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for TimestampPolicy {
    type Err = InvalidPolicy;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "fixed" => Ok(TimestampPolicy::Fixed),
            "source" => Ok(TimestampPolicy::Source),
            "now" => Ok(TimestampPolicy::Now),
            other => Err(InvalidPolicy(other.to_string())),
        }
    }
}

#[derive(Debug, Error)]
#[error("invalid timestamp policy '{0}' (expected one of: fixed, source, now)")]
pub struct InvalidPolicy(String);

#[derive(Debug, Clone)]
pub struct ProjectManifest {
    pub source: Utf8PathBuf,
    pub name: String,
    pub timestamp: Option<NaiveDateTime>,
    pub timestamp_policy: TimestampPolicy,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl ProjectManifest {
    pub fn effective_policy(&self, override_policy: Option<TimestampPolicy>) -> TimestampPolicy {
        override_policy.unwrap_or(self.timestamp_policy)
    }

    pub fn resolved_timestamp(&self, policy: TimestampPolicy) -> NaiveDateTime {
        match (self.timestamp, policy) {
            (Some(ts), _) => ts,
            (None, TimestampPolicy::Fixed) => fixed_timestamp(),
            (None, TimestampPolicy::Now) => current_timestamp(),
            (None, TimestampPolicy::Source) => fixed_timestamp(),
        }
    }
}

pub fn load_manifest(path: &Path) -> Result<ProjectManifest, PsuPackError> {
    let manifest_path = path.join("psu.toml");
    if !manifest_path.exists() {
        return Err(PsuPackError::MissingManifest {
            root: path.to_path_buf(),
        });
    }

    let contents = fs::read_to_string(&manifest_path).map_err(|source| PsuPackError::Io {
        path: manifest_path.clone(),
        source,
    })?;

    let raw: RawManifest = toml::from_str(&contents).map_err(|source| PsuPackError::Toml {
        path: manifest_path.clone(),
        source,
    })?;

    let mut issues = Vec::new();

    let config = match raw.config {
        Some(config) => config,
        None => {
            issues.push(
                ValidationIssue::error("config", "missing [config] section")
                    .with_help("Add a [config] table with at least a name field."),
            );
            return Err(PsuPackError::InvalidConfig {
                path: Utf8PathBuf::from_path_buf(manifest_path.clone())
                    .expect("manifest path must be valid UTF-8"),
                issues,
            });
        }
    };

    let name = match config.name {
        Some(name) if !name.trim().is_empty() => name,
        Some(_) => {
            issues.push(
                ValidationIssue::error("config.name", "name cannot be blank")
                    .with_help("Provide a memory card folder name (e.g. 'MY SAVE')."),
            );
            String::new()
        }
        None => {
            issues.push(
                ValidationIssue::error("config.name", "missing required field")
                    .with_help("Set config.name to the folder name shown on the memory card."),
            );
            String::new()
        }
    };

    let timestamp = match config.timestamp.as_deref() {
        Some(raw_value) if raw_value.trim().is_empty() => {
            issues.push(
                ValidationIssue::error("config.timestamp", "timestamp cannot be empty")
                    .with_help("Remove the timestamp field or provide a value."),
            );
            None
        }
        Some(raw_value) => match parse_timestamp(raw_value) {
            Ok(ts) => Some(ts),
            Err(message) => {
                issues.push(
                    ValidationIssue::error(
                        "config.timestamp",
                        format!("{message} (value: {raw_value:?})"),
                    )
                    .with_help(
                        "Use RFC 3339 (e.g. 2001-02-03T04:05:06Z) or 'YYYY-MM-DD HH:MM:SS'.",
                    ),
                );
                None
            }
        },
        None => None,
    };

    let timestamp_policy = match config.timestamp_policy.as_deref() {
        Some(value) => match value.parse() {
            Ok(policy) => policy,
            Err(err) => {
                issues.push(
                    ValidationIssue::error("config.timestamp_policy", format!("{err}"))
                        .with_help("Supported values: fixed, source, now."),
                );
                TimestampPolicy::Fixed
            }
        },
        None => TimestampPolicy::Fixed,
    };

    let include = config.include.unwrap_or_default();
    let exclude = config.exclude.unwrap_or_default();

    validate_name(&name, &mut issues);
    validate_include_exclude(&include, &exclude, &mut issues);

    if !issues.is_empty() {
        return Err(PsuPackError::InvalidConfig {
            path: Utf8PathBuf::from_path_buf(manifest_path.clone())
                .expect("manifest path must be valid UTF-8"),
            issues,
        });
    }

    Ok(ProjectManifest {
        source: Utf8PathBuf::from_path_buf(manifest_path).expect("manifest paths must be utf-8"),
        name,
        timestamp,
        timestamp_policy,
        include,
        exclude,
    })
}

fn validate_name(name: &str, issues: &mut Vec<ValidationIssue>) {
    if name.len() > 32 {
        issues.push(
            ValidationIssue::error("config.name", "name exceeds 32 characters")
                .with_help("Use up to 32 ASCII characters to match PS2 limits."),
        );
    }

    if !name
        .chars()
        .all(|c| matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | ' ' | '.'))
    {
        issues.push(
            ValidationIssue::error("config.name", "name contains unsupported characters")
                .with_help("Allowed characters: A-Z, 0-9, space, '-', '_', '.'."),
        );
    }
}

fn validate_include_exclude(
    include: &[String],
    exclude: &[String],
    issues: &mut Vec<ValidationIssue>,
) {
    if !include.is_empty() && !exclude.is_empty() {
        issues.push(
            ValidationIssue::error(
                "config.include",
                "include and exclude cannot be used together",
            )
            .with_help("Remove one list so that only include or exclude is defined."),
        );
    }

    let mut seen_include = std::collections::BTreeSet::new();
    for (idx, value) in include.iter().enumerate() {
        if value.trim().is_empty() {
            issues.push(ValidationIssue::error(
                format!("config.include[{idx}]"),
                "entry cannot be empty",
            ));
        }
        if value.contains(['/', '\\']) {
            issues.push(
                ValidationIssue::error(
                    format!("config.include[{idx}]"),
                    "subdirectories are not supported",
                )
                .with_help("Place files next to psu.toml and reference only the file name."),
            );
        }
        if !seen_include.insert(value.to_ascii_uppercase()) {
            issues.push(
                ValidationIssue::error(format!("config.include[{idx}]"), "duplicate entry")
                    .with_help("Remove repeated file names; entries are case-insensitive."),
            );
        }
    }

    let mut seen_exclude = std::collections::BTreeSet::new();
    for (idx, value) in exclude.iter().enumerate() {
        if value.trim().is_empty() {
            issues.push(ValidationIssue::error(
                format!("config.exclude[{idx}]"),
                "entry cannot be empty",
            ));
        }
        if value.contains(['/', '\\']) {
            issues.push(
                ValidationIssue::error(
                    format!("config.exclude[{idx}]"),
                    "subdirectories are not supported",
                )
                .with_help("Place files next to psu.toml and reference only the file name."),
            );
        }
        if !seen_exclude.insert(value.to_ascii_uppercase()) {
            issues.push(
                ValidationIssue::error(format!("config.exclude[{idx}]"), "duplicate entry")
                    .with_help("Remove repeated file names; entries are case-insensitive."),
            );
        }
    }
}

fn parse_timestamp(raw: &str) -> Result<NaiveDateTime, String> {
    if let Ok(date) = DateTime::parse_from_rfc3339(raw) {
        return Ok(date.naive_utc());
    }

    if let Ok(date) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        return Ok(date);
    }

    if let Ok(date) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
        return Ok(date);
    }

    Err("invalid timestamp format".to_string())
}

fn current_timestamp() -> NaiveDateTime {
    DateTime::<Utc>::from(std::time::SystemTime::now())
        .naive_utc()
        .with_nanosecond(0)
        .expect("truncate nanoseconds")
}

#[derive(Debug, Deserialize)]
struct RawManifest {
    config: Option<RawConfig>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    name: Option<String>,
    timestamp: Option<String>,
    timestamp_policy: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PsuPackError;
    use tempfile::TempDir;

    fn write_manifest(dir: &TempDir, contents: &str) {
        std::fs::write(dir.path().join("psu.toml"), contents).expect("write manifest");
    }

    #[test]
    fn valid_manifest_loads() {
        let dir = TempDir::new().unwrap();
        write_manifest(
            &dir,
            r#"
            [config]
            name = "GOOD SAVE"
            timestamp = "2001-02-03T04:05:06Z"
            include = ["ICON.SYS"]
            "#,
        );

        let manifest = load_manifest(dir.path()).expect("manifest should load");
        assert_eq!(manifest.name, "GOOD SAVE");
        assert_eq!(manifest.include, vec!["ICON.SYS".to_string()]);
    }

    #[test]
    fn invalid_manifest_reports_issues() {
        let dir = TempDir::new().unwrap();
        write_manifest(
            &dir,
            r#"
            [config]
            name = "invalid*name"
            include = ["file.bin", "file.bin"]
            exclude = ["other.bin"]
            "#,
        );

        let err = load_manifest(dir.path()).expect_err("manifest should fail");
        if let PsuPackError::InvalidConfig { issues, .. } = err {
            assert!(issues.iter().any(|issue| issue.field == "config.name"));
            assert!(issues
                .iter()
                .any(|issue| issue.field.contains("config.include")));
            assert!(issues.iter().any(|issue| issue.field == "config.include"));
        } else {
            panic!("expected invalid config error");
        }
    }
}
