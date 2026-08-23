# CI/CD & Deployment

This project uses the canonical CI workflow shared byte-identically by all
rs* repositories. The single workflow file is
`.github/workflows/ci.yml`; its canonical copy lives in the `rsconstruct`
repository and is synced out — do not edit it here.

## The test job

Every push runs build, clippy (`-D warnings`), and the test suite. External
tools the tests need are installed by `scripts/ci-install-tools.sh`, a
repo-owned hook the workflow requires in every repo (an explicit no-op where
nothing is needed).

## Documentation Deployment

The documentation is built with `mdBook` from `docs/` (`docs/book.toml`,
sources in `docs/src/`, output in `docs/book/`, which is ignored by Git) and
deployed to GitHub Pages from a temporary build artifact using
`actions/deploy-pages` — no separate `gh-pages` branch. Deployment runs on
version tags and on manual `workflow_dispatch`. The `release-info.md` page is
populated with version, tag, commit, and workflow details at deploy time.

## Binary Releases

Releases are triggered by pushing a version tag (e.g., `v0.2.1`).

### Release Strategy
- **Platform Support**: The workflow builds for:
  - Linux (x86_64, aarch64)
  - macOS (x86_64, aarch64)
- **Automated Assets**: Binaries are automatically renamed with platform
  suffixes and attached to the GitHub Release.
- **Notes Generation**: Release notes are automatically generated based on
  commit history.
- A release never ships from a commit whose tests fail: the release jobs
  depend on the test job.

## Versioning

This project uses `cargo-release` for version management.

```bash
# Example: Bump patch version and push tag
cargo release patch --execute
```

The `release.toml` file in the root directory contains the configuration for
`cargo-release`, ensuring consistent tag formats and signing.
