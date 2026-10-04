# TOFIX

Findings from a code scan on 2026-10-04.

## Medium

- `src/wikipedia.rs:155` - `cmd_collect` loops `while total < target_count` with no exit on failure; `fetch_wikipedia_paragraphs_batch` (line 96) returns an empty vec on any network/JSON error, so `rstype wikipedia download` offline spins forever every 100 ms. Return a `Result` from the fetch and abort after repeated failures or empty batches.
- `src/main.rs:302` - any `?` error from `render`, `event::poll` or `event::read` (and any panic) returns before `disable_raw_mode`/`LeaveAlternateScreen` at lines 318-319, leaving the user's terminal in raw mode on the alternate screen; restore the terminal in a guard/`Drop` (or `ratatui::init`/`restore`) and a panic hook.
- `_site/index.html:1` - the whole `_site/` mdBook output (about 50 files) is committed although `.gitignore:218` ignores `/_site/` and CI builds the book from `docs/` into `docs/book` (`.github/workflows/ci.yml:213`); it is stale (no page for `docs/src/release-info.md`). `git rm -r --cached _site`.
- `README.md:43` - claims builds for Windows, and line 49 says pre-built binaries exist for "Linux, macOS, and Windows", but the release matrix in `.github/workflows/ci.yml:145-155` builds only Linux and macOS targets; drop Windows from the README (and list the macOS download names, which the install section omits).

## Low

- `src/dict.rs:1166` - CLI failures (`dict install`, `dict remove`, `stats clear` at `src/main.rs:277`, ...) print to stderr and return, so the process exits 0; propagate errors so scripts see a non-zero status.
- `Cargo.toml:17` - `clap = "4.6.0"` is the only dependency with a patch-level minimum, with no comment why; use `"4"` like the other entries, letting `Cargo.lock` hold the exact version.
