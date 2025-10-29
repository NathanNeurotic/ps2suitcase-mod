use std::path::{Path, PathBuf};
use std::time::SystemTime;

use psu_packer::{
    load_manifest, pack_project, PackOptions, PackResult, ProjectManifest, PsuPackError,
    TimestampPolicy, ValidationIssue,
};

pub struct ProjectState {
    root: PathBuf,
    pub manifest: Option<ProjectManifest>,
    pub validation: Vec<ValidationIssue>,
    pub last_error: Option<String>,
    pub preview: Option<PackResult>,
    last_manifest_mtime: Option<SystemTime>,
}

impl ProjectState {
    pub fn open(root: PathBuf) -> Self {
        let mut state = Self {
            root,
            manifest: None,
            validation: Vec::new(),
            last_error: None,
            preview: None,
            last_manifest_mtime: None,
        };
        state.refresh(true);
        state
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn refresh(&mut self, force: bool) {
        let manifest_path = self.root.join("psu.toml");
        match std::fs::metadata(&manifest_path) {
            Ok(metadata) => {
                let new_mtime = metadata.modified().ok();
                let changed = match (self.last_manifest_mtime, new_mtime) {
                    (Some(old), Some(new)) => old != new,
                    _ => true,
                };
                if !force && !changed {
                    return;
                }
                self.last_manifest_mtime = new_mtime;
            }
            Err(err) => {
                self.last_error = Some(format!("{}", err));
                self.manifest = None;
                self.validation.clear();
                self.preview = None;
                return;
            }
        }

        match load_manifest(&self.root) {
            Ok(manifest) => {
                self.validation.clear();
                self.last_error = None;
                self.preview = Some(self.build_preview(&manifest));
                self.manifest = Some(manifest);
            }
            Err(PsuPackError::InvalidConfig { issues, .. }) => {
                self.manifest = None;
                self.preview = None;
                self.last_error = None;
                self.validation = issues;
            }
            Err(err) => {
                self.manifest = None;
                self.preview = None;
                self.validation.clear();
                self.last_error = Some(err.to_string());
            }
        }
    }

    pub fn export(
        &self,
        override_policy: Option<TimestampPolicy>,
        verify: bool,
        output: &Path,
    ) -> Result<(), PsuPackError> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| PsuPackError::MissingManifest {
                root: self.root.clone(),
            })?;
        let pack = pack_project(
            &self.root,
            manifest,
            PackOptions {
                timestamp_policy: override_policy,
                verify,
            },
        )?;
        std::fs::write(output, &pack.bytes).map_err(|source| PsuPackError::Io {
            path: output.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    fn build_preview(&self, manifest: &ProjectManifest) -> PackResult {
        match pack_project(
            &self.root,
            manifest,
            PackOptions {
                timestamp_policy: None,
                verify: false,
            },
        ) {
            Ok(mut result) => {
                result.bytes.clear();
                result
            }
            Err(_) => PackResult {
                bytes: Vec::new(),
                files: Vec::new(),
                policy: manifest.timestamp_policy,
            },
        }
    }
}
