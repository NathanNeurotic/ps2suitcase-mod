use std::io;
use std::path::PathBuf;

use camino::Utf8PathBuf;

use crate::manifest::ValidationIssue;

#[derive(Debug, thiserror::Error)]
pub enum PsuPackError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path}: {source}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("missing psu.toml in {root}")]
    MissingManifest { root: PathBuf },
    #[error("configuration errors in {path}")]
    InvalidConfig {
        path: Utf8PathBuf,
        issues: Vec<ValidationIssue>,
    },
    #[error("referenced file not found: {path}")]
    MissingFile { path: PathBuf },
    #[error("no files matched the manifest selection in {root}")]
    EmptySelection { root: PathBuf },
    #[error("verification failed: {message}")]
    VerificationFailed { message: String },
}

impl PsuPackError {
    pub fn issues(&self) -> Option<&[ValidationIssue]> {
        match self {
            PsuPackError::InvalidConfig { issues, .. } => Some(issues),
            _ => None,
        }
    }
}
