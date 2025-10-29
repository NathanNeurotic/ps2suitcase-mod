# PS2 Suitcase (mod)

![Continuous Integration](https://github.com/NathanNeurotic/ps2suitcase-mod/actions/workflows/ci.yml/badge.svg)

Deterministic tooling for building PlayStation 2 PSU memory card packages. This fork streamlines
command-line packing and the "Suitcase" GUI for reliable automation driven by GitHub Actions.

## Workspace overview

| Crate | Description |
|-------|-------------|
| [`psu-packer`](crates/psu-packer) | Deterministic CLI and library for turning a folder described by `psu.toml` into a PSU archive. |
| [`suitcase`](crates/suitcase) | Minimal GUI wrapper around the packer with live validation and export. |
| [`ps2-filetypes`](crates/ps2-filetypes) | Shared PSU primitives and serialization helpers. |

The default build target is `psu-packer`; run `cargo run -p psu-packer -- --help` for CLI usage.

## `psu.toml` schema

Each project folder contains a `psu.toml` manifest describing the save contents. The schema is
strictly validated before packing.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `config.name` | string | ✅ | Display name on the memory card (max 32 ASCII characters). |
| `config.timestamp` | string | ❌ | RFC 3339 (`2003-01-02T03:04:05Z`) or `YYYY-MM-DD HH:MM:SS`. Defaults to the fixed baseline if omitted. |
| `config.timestamp_policy` | string | ❌ | Overrides the packing policy. Supported values: `fixed` (default), `source`, `now`. |
| `config.include` | array of strings | ❌ | Explicit file list. File names are case-insensitive and must sit next to `psu.toml`. |
| `config.exclude` | array of strings | ❌ | Files to skip when including everything else. Mutually exclusive with `include`. |

Validation highlights:

* Names must be 32 characters or fewer and use `[A-Za-z0-9 ._-]` only.
* `include` and `exclude` cannot be set together and never allow duplicates.
* Sub-directories are rejected to keep PSU layouts flat and deterministic.

Example manifests live in [`assets/templates`](assets/templates) and are heavily commented.
Copy one into your project folder as a starting point.

## Timestamp policies

`psu-packer` supports three deterministic timestamp modes:

* **`fixed`** (default) – use the baseline `2000-01-01T00:00:00` unless overridden by
  `config.timestamp`.
* **`source`** – read each file's metadata (falling back to the baseline if unavailable).
* **`now`** – stamp the archive with the current UTC second (useful for ad-hoc builds).

Override the manifest policy with the CLI flag `--timestamp-policy <fixed|source|now>` or the GUI
combo box when exporting.

## Deterministic tests & golden files

Golden fixtures live under [`test/fixtures`](test/fixtures). Each pack is hashed and compared against
[`test/golden`](test/golden) during `cargo test`. When a legitimate layout change occurs, update the
stored hashes with:

```bash
UPDATE_GOLDEN=1 cargo test -p psu-packer
```

Two sample projects (`minimal` and `exclude`) exercise both include and exclude flows.

## GitHub Actions workflows

* [`ci.yml`](.github/workflows/ci.yml) – Runs `fmt`, `clippy -D warnings`, `build`, and `test` on
  Linux, macOS, and Windows for every push and pull request.
* [`release-build.yml`](.github/workflows/release-build.yml) – Manual workflow that builds the
  selected crates on demand and uploads binaries per operating system. Choose crates, profile, and
  optional feature flags directly from the dispatch form.

CI is the only supported build path; local builds are optional.

## Release process

1. Trigger **On-demand Builds** to produce release candidates (`profile=release`).
2. Download platform archives, smoke-test the GUI, and confirm deterministic hashes.
3. Draft release notes using [.github/RELEASE_TEMPLATE.md](.github/RELEASE_TEMPLATE.md) and attach
   the generated artifacts.

## Suitcase GUI quick start

1. Launch the `suitcase` binary from the Actions artifact or `cargo run -p suitcase` locally.
2. Use **Open folder** to select a project directory with `psu.toml` and assets.
3. Fix any validation issues listed in red.
4. Export a PSU with the desired timestamp policy override and optional verification.

The preview lists every file that will be packed and highlights when an `icon.sys` asset is present
(with a placeholder message until richer previews land).
