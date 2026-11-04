# Changelog

All notable changes to this project will be documented in this file.

## 0.0.7 - Unreleased

### Performance

- **Zero-allocation line counting and clean-file bypass**: Bypassed `split_lines` entirely for clean files in HEAD when not filtering blank lines, and replaced `split_lines().len()` with a zero-allocation `count_lines` byte scanner.
- **Zero-copy CRLF normalization**: Refactored `normalize_crlf` to return `Cow<'_, [u8]>` with a fast-path scan, eliminating full-file heap allocations on Unix and CRLF-free repositories.
- **Author interning and `Arc<str>` caching**: Cached commit authors as `Arc<str>` across worker threads and interned authors per file into `Vec<Option<u32>>`, reducing HEAD tracking slot memory from 24 bytes/line to 4 bytes/line (~83% memory reduction) and eliminating `String` clones and `HashMap` lookups from the hot per-line loop.
- **Repository single-open threading**: Reused the discovered `gix::Repository` instance from tracked file collection in worker threads instead of opening the repository twice.
- **Bounded event channels**: Switched the blame event channel from unbounded to bounded (`workers * 4`) to prevent unbounded memory growth on monorepos.
- **Path allocation reductions**: Refactored `path_to_git_string` and `display_path` to build directly into pre-allocated strings without intermediate `Vec<String>` joins.

### Changed

- **Domain error types (`thiserror`)**: Introduced structured domain error enums (`GitError`, `BlameError`, `BlameFileError`, `RunError`) in `src/error.rs`, preserving underlying error sources while guaranteeing exact user-facing error strings and exit codes.
- **Typed blame event warnings**: `BlameEvent::Warning` now carries `&BlameFileError` directly to eliminate intermediate string formatting in worker event loops.
- **Worker thread state bundling**: Encapsulated HEAD commit and tree references into `HeadState` to reduce parameter counts and improve thread-local clarity.

### Fixed

- **Filter zero-line authors**: Excluded authors with 0 attributed lines from table and JSON outputs so deleted or fully filtered worktree authors do not render with `0.0%` bars.
- **Panic safety and worker resilience**: Documented worker panic safety invariants and ensured per-file panics cleanly convert into structured `BlameFileError::WorkerPanic` warnings without crashing worker threads.

## 0.0.6 - 2026-10-30

### Fixed

- Restore the `git-blamr` binary target so `cargo install blamr`, `cargo binstall blamr`, and archive extractions install both `blamr` and `git-blamr` directly.
- Increase libgit2 tree and commit object cache limits to prevent cache thrashing and stalls on large repositories.

## 0.0.5 - 2026-10-30

- Updated transitive dependecies

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
