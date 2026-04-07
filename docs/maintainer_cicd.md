# CI/CD & Deployment

This project uses a modern, streamlined GitHub Actions workflow for both documentation deployment and binary releases.

## Documentation Deployment

The documentation is built using `mdBook` and deployed directly to GitHub Pages without the need for a separate `gh-pages` branch.

### Key Features of the Workflow
- **Stateless Deployment**: The site is deployed from a temporary build artifact using the `actions/deploy-pages` action.
- **Root Configuration**: `book.toml` resides in the project root for simplicity.
- **Direct Source Paths**: Source files are located in `docs/`, and the output is generated in `_site/` (which is ignored by Git).
- **Automated Builds**: Any push to the `master` branch triggers a re-build and re-deployment of the documentation.

### Workflow File
The configuration is defined in `.github/workflows/docs.yml`.

---

## Binary Releases

Releases are triggered by pushing a version tag (e.g., `v0.2.1`).

### Release Strategy
- **Platform Support**: The workflow currently builds for:
  - Linux (x86_64, aarch64)
  - macOS (x86_64, aarch64)
- **Automated Assets**: Binaries are automatically renamed with platform suffixes and attached to the GitHub Release.
- **Notes Generation**: Release notes are automatically generated based on commit history.

### Workflow File
The configuration is defined in `.github/workflows/release.yml`.

---

## Versioning

This project uses `cargo-release` for version management.

```bash
# Example: Bump patch version and push tag
cargo release patch --execute
```

The `release.toml` file in the root directory contains the configuration for `cargo-release`, ensuring consistent tag formats and signing.
