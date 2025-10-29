use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use camino::Utf8PathBuf;
use chrono::{DateTime, NaiveDateTime, Timelike, Utc};
use ps2_filetypes::{PSUEntry, PSUEntryKind, PSUWriter, DIR_ID, FILE_ID, PSU};

use crate::error::PsuPackError;
use crate::manifest::{ProjectManifest, TimestampPolicy};
use crate::verify::verify_bytes;

#[derive(Debug, Clone, Default)]
pub struct PackOptions {
    pub timestamp_policy: Option<TimestampPolicy>,
    pub verify: bool,
}

#[derive(Debug, Clone)]
pub struct PackedFile {
    pub name: String,
    pub source: PathBuf,
    pub size: u64,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct PackResult {
    pub bytes: Vec<u8>,
    pub files: Vec<PackedFile>,
    pub policy: TimestampPolicy,
}

pub fn pack_project(
    root: &Path,
    manifest: &ProjectManifest,
    options: PackOptions,
) -> Result<PackResult, PsuPackError> {
    let policy = manifest.effective_policy(options.timestamp_policy);
    let project_timestamp = manifest.resolved_timestamp(policy);

    let files = select_files(root, manifest)?;
    if files.is_empty() {
        return Err(PsuPackError::EmptySelection {
            root: root.to_path_buf(),
        });
    }

    let mut psu = PSU::default();
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: files.len() as u32 + 2,
        created: project_timestamp,
        sector: 0,
        modified: project_timestamp,
        name: manifest.name.clone(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: 0,
        created: project_timestamp,
        sector: 0,
        modified: project_timestamp,
        name: ".".to_string(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: 0,
        created: project_timestamp,
        sector: 0,
        modified: project_timestamp,
        name: "..".to_string(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });

    let mut packed_files = Vec::with_capacity(files.len());

    for file in files {
        let metadata = fs::metadata(&file).map_err(|source| PsuPackError::Io {
            path: file.clone(),
            source,
        })?;
        if !metadata.is_file() {
            return Err(PsuPackError::MissingFile { path: file });
        }

        let timestamp = match policy {
            TimestampPolicy::Fixed => project_timestamp,
            TimestampPolicy::Now => project_timestamp,
            TimestampPolicy::Source => metadata_timestamp(&metadata).unwrap_or(project_timestamp),
        };

        let contents = fs::read(&file).map_err(|source| PsuPackError::Io {
            path: file.clone(),
            source,
        })?;

        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| PsuPackError::MissingFile { path: file.clone() })?
            .to_string();

        psu.entries.push(PSUEntry {
            id: FILE_ID,
            size: contents.len() as u32,
            created: timestamp,
            sector: 0,
            modified: timestamp,
            name: name.clone(),
            kind: PSUEntryKind::File,
            contents: Some(contents),
        });

        packed_files.push(PackedFile {
            name,
            source: file,
            size: metadata.len(),
            timestamp,
        });
    }

    let bytes = PSUWriter::new(psu)
        .to_bytes()
        .map_err(|source| PsuPackError::Io {
            path: root.to_path_buf(),
            source,
        })?;

    if options.verify {
        verify_bytes(&bytes, packed_files.len())
            .map_err(|err| PsuPackError::VerificationFailed { message: err })?;
    }

    Ok(PackResult {
        bytes,
        files: packed_files,
        policy,
    })
}

fn select_files(root: &Path, manifest: &ProjectManifest) -> Result<Vec<PathBuf>, PsuPackError> {
    if !manifest.include.is_empty() {
        collect_included(root, &manifest.include)
    } else {
        collect_all(root, &manifest.exclude)
    }
}

fn collect_included(root: &Path, include: &[String]) -> Result<Vec<PathBuf>, PsuPackError> {
    let mut files = Vec::with_capacity(include.len());
    let mut seen = BTreeSet::new();

    for entry in include {
        if !seen.insert(entry.to_ascii_uppercase()) {
            continue;
        }
        let path = root.join(entry);
        if !path.exists() {
            return Err(PsuPackError::MissingFile { path });
        }
        if !path.is_file() {
            return Err(PsuPackError::MissingFile { path });
        }
        files.push(path);
    }

    files.sort_by(|a, b| compare_paths(a, b));

    Ok(files)
}

fn collect_all(root: &Path, exclude: &[String]) -> Result<Vec<PathBuf>, PsuPackError> {
    let exclude: BTreeSet<_> = exclude.iter().map(|s| s.to_ascii_uppercase()).collect();
    let mut files = Vec::new();

    let entries = fs::read_dir(root).map_err(|source| PsuPackError::Io {
        path: root.to_path_buf(),
        source,
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("psu.toml"))
            .unwrap_or(false)
        {
            continue;
        }

        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            if exclude.contains(&name.to_ascii_uppercase()) {
                continue;
            }
        }

        files.push(path);
    }

    files.sort_by(|a, b| compare_paths(a, b));

    Ok(files)
}

fn compare_paths(a: &PathBuf, b: &PathBuf) -> std::cmp::Ordering {
    let left = Utf8PathBuf::from_path_buf(a.clone()).expect("file names must be utf-8");
    let right = Utf8PathBuf::from_path_buf(b.clone()).expect("file names must be utf-8");
    left.as_str().cmp(right.as_str())
}

fn metadata_timestamp(metadata: &fs::Metadata) -> Option<NaiveDateTime> {
    metadata
        .modified()
        .or_else(|_| metadata.created())
        .ok()
        .and_then(|time| system_time_to_naive(time).ok())
}

fn system_time_to_naive(time: SystemTime) -> io::Result<NaiveDateTime> {
    let datetime = DateTime::<Utc>::from(time);
    Ok(datetime
        .naive_utc()
        .with_nanosecond(0)
        .expect("truncate nanoseconds"))
}
