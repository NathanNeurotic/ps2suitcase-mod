use std::fs;
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use psu_packer::{load_manifest, pack_project, PackOptions};
use sha2::{Digest, Sha256};

static WORKSPACE_ROOT: Lazy<PathBuf> = Lazy::new(|| {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("workspace root")
});

#[test]
fn minimal_fixture_matches_golden() {
    run_fixture("minimal");
}

#[test]
fn exclude_fixture_matches_golden() {
    run_fixture("exclude");
}

fn run_fixture(name: &str) {
    let project_dir = WORKSPACE_ROOT.join("test/fixtures").join(name);
    let manifest = load_manifest(&project_dir).expect("manifest should load");

    let pack = pack_project(
        &project_dir,
        &manifest,
        PackOptions {
            timestamp_policy: None,
            verify: true,
        },
    )
    .expect("packing should succeed");

    let digest = format_hash(&pack.bytes);
    let golden_path = WORKSPACE_ROOT
        .join("test/golden")
        .join(format!("{name}.sha256"));

    if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        fs::write(&golden_path, format!("{digest}\n")).expect("failed to update golden");
        return;
    }

    let expected = fs::read_to_string(&golden_path)
        .unwrap_or_else(|_| panic!("missing golden hash: {}", golden_path.display()))
        .trim()
        .to_string();

    assert_eq!(expected, digest, "golden hash mismatch for {name}");
}

fn format_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex::encode(digest)
}
