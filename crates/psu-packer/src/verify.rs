use std::collections::BTreeSet;

use ps2_filetypes::{PSUEntryKind, PSU};

pub fn verify_bytes(bytes: &[u8], expected_files: usize) -> Result<(), String> {
    let psu = PSU::parse(bytes.to_vec())
        .map_err(|err| format!("failed to parse generated PSU: {err}"))?;

    if psu.entries.len() != expected_files + 3 {
        return Err(format!(
            "expected {} entries ({} files + 3 directory markers), found {}",
            expected_files + 3,
            expected_files,
            psu.entries.len()
        ));
    }

    let mut names = BTreeSet::new();
    for entry in psu.entries.iter().skip(3) {
        if entry.kind != PSUEntryKind::File {
            return Err(format!("unexpected non-file entry '{}'", entry.name));
        }
        if entry.contents.is_none() {
            return Err(format!("file '{}' is missing contents", entry.name));
        }
        if (entry.contents.as_ref().unwrap().len() as u32) != entry.size {
            return Err(format!(
                "file '{}' declared size {} but stored {} bytes",
                entry.name,
                entry.size,
                entry.contents.as_ref().unwrap().len()
            ));
        }
        if !names.insert(entry.name.clone()) {
            return Err(format!("duplicate entry '{}' detected", entry.name));
        }
    }

    Ok(())
}
