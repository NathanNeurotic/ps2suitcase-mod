pub mod error;
pub mod manifest;
pub mod packer;
pub mod verify;

pub use chrono::{NaiveDate, NaiveDateTime};

pub use error::PsuPackError;
pub use manifest::{load_manifest, ProjectManifest, TimestampPolicy, ValidationIssue};
pub use packer::{pack_project, PackOptions, PackResult, PackedFile};
pub use verify::verify_bytes;

pub fn fixed_timestamp() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2000, 1, 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .expect("valid fixed timestamp baseline")
}
