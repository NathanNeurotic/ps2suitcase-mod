# Contributing

This project uses GitHub Actions as the primary build loop. Local builds are optional.

## Workflow

1. Fork the repository and branch from `main`.
2. Make your changes and push to the branch.
3. Open a pull request – the [Continuous Integration](.github/workflows/ci.yml) workflow will run
   `cargo fmt`, `cargo clippy -D warnings`, `cargo build --workspace`, and `cargo test --workspace`
   on Linux, macOS, and Windows.
4. Review the CI results and iterate by pushing more commits. Keep PRs small and focused.

## Deterministic checks

* Use the fixtures in `test/fixtures` to reproduce CI behaviour. Hashes are stored in
  `test/golden`.
* Update hashes with `UPDATE_GOLDEN=1 cargo test -p psu-packer` when intentional output changes are
  made.

## Releases

Use the [On-demand Builds](.github/workflows/release-build.yml) workflow to produce binaries for
manual testing. The generated artifacts can be attached to a GitHub release using the
[`RELEASE_TEMPLATE.md`](.github/RELEASE_TEMPLATE.md) checklist.
