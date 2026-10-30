# Changelog

All notable changes to this project will be documented in this file.

## 0.0.4 - 2026-10-30

### Changed

- Ship a single `blamr` binary; installers create a `git-blamr` alias for `git blamr`.
- Publish fully static musl archives for Linux and resolve glibc hosts to them in
  `cargo-binstall`.
- Do not ignore whitespace-only changes by default; use `-ignore-whitespace` to opt in.
- Write progress to stderr, throttled and cleared before warnings, with `-no-progress` to disable it.
- Add `-ascii` and automatic Unicode/console-encoding detection for Windows terminals.
- Write help to stdout and usage errors to stderr.
- Reduce per-line work and enable fast release binaries with LTO and stripped symbols.

### Fixed

- Attribute current files to `Not Committed Yet` when the repository has an unborn HEAD instead of
  warning and skipping them.
- Keep table output free of a leading blank line.

## 0.0.3 - 2026-10-27

### Changed

- Rewrote the project in Rust, replacing the Go implementation.
- Statically linked `libgit2`.
- Added cargo-dist release archives, shell and PowerShell installers, crates.io publishing, and
  cargo-binstall metadata.
- `v0.0.2` is the final Go release.

### Fixed

- Count conflicted index entries once.
- Preserve explicit symlink paths and treat symlinks as text blobs.
- Avoid spawning more workers than files.
- Correctly detect broken stdout pipes on Windows.
