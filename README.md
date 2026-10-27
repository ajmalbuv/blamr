# blamr

> Concurrent git-blame author attribution and line statistics, written in Rust.

[![crates.io](https://img.shields.io/crates/v/blamr.svg)](https://crates.io/crates/blamr)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`blamr` is a local-only Git authorship and line-statistics tool. It embeds libgit2, so the
released binaries never invoke a `git` executable and do not require Git to be installed.

Both binaries are included in every release:

- `blamr`: standalone command.
- `git-blamr`: Git subcommand entry point, invoked as `git blamr`.

## Installation

### Prebuilt binaries

Download the archive for your platform from
[GitHub Releases](https://github.com/ajmalbuv/blamr/releases), extract it, and put both
`blamr` and `git-blamr` on `PATH`.

Linux and macOS installers are also published with every release:

```console
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ajmalbuv/blamr/releases/latest/download/blamr-installer.sh | sh
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ajmalbuv/blamr/releases/latest/download/blamr-installer.ps1 | iex"
```

Windows archives are `.zip`; Linux and macOS archives are `.tar.gz`. Linux builds use musl and
are statically linked. macOS x86_64 and arm64 archives are separate.

### cargo-binstall

```console
cargo binstall blamr
```

This installs the prebuilt binaries from GitHub Releases without compiling.

### crates.io

```console
cargo install blamr
```

Compiling from source requires CMake and a C toolchain for the vendored libgit2.

The binaries are unsigned. Windows SmartScreen and macOS Gatekeeper may warn about downloaded
binaries. There is no notarization, code signing, or WinGet manifest.

Git is not required at runtime. Custom clean/smudge filters and textconv drivers are not
emulated.

## Usage

Run inside a repository to analyze every tracked text file:

```console
$ blamr
$ git blamr
```

```text
─────────────────────────────────────────────
Author                       Lines        %
─────────────────────────────────────────────
Alice Smith                   8200    65.9%  █████████████
Bob Jones                     4250    34.1%  ██████
─────────────────────────────────────────────
Total                        12450
```

Scope the analysis to files or directories:

```console
blamr ./src ./internal
blamr frontend/
blamr -- -dash-prefixed-file.txt
```

Only selected file types:

```console
blamr -ext .rs,.ts,.py
```

Attribution and output options:

```console
blamr -email          # group by email address
blamr -ignore-blank   # skip empty and whitespace-only lines
blamr -committed-only # exclude working-tree edits
blamr -json           # structured JSON
blamr -j 4            # worker count
```

### Flags

| Flag | Default | Description |
| :--- | :--- | :--- |
| `-ext <list>` | all text files | Comma-separated extensions, e.g. `.rs,.ts`. Bypasses generated/minified/binary filtering. |
| `-j <num>` | CPU count | Number of blame workers. Values below 1 use the default. |
| `-email` | `false` | Group by author email instead of name. |
| `-ignore-blank` | `false` | Ignore empty and whitespace-only lines. |
| `-committed-only` | `false` | Exclude uncommitted working-tree changes. |
| `-json` | `false` | Emit JSON with two-space indentation. |
| `-v`, `-version` | `false` | Print the version and exit. |
| `-h`, `-help` | `false` | Print help to stderr and exit 0. |

Flags may appear before or after paths. `--` ends flag parsing. `-ext` and `-j` are last-wins.
Exit status is 0 on success, 1 for execution failures, and 2 for usage errors.

### JSON

```json
{
  "total_files": 42,
  "total_lines": 12450,
  "authors": [
    {
      "author": "Alice Smith",
      "lines": 8200,
      "percentage": 65.86
    },
    {
      "author": "Bob Jones",
      "lines": 4250,
      "percentage": 34.14
    }
  ]
}
```

`authors` is always an array. If every candidate file is filtered or blame fails, `total_files`
is 0 and the array is empty.

## Behavior and limitations

`blamr` supports:

- standard repositories, linked worktrees, subdirectory invocation, and `GIT_DIR`,
  `GIT_WORK_TREE`, and `GIT_INDEX_FILE`;
- staged files, unborn branches, merge/rebase conflicts, add/add and delete/modify conflicts;
- `.mailmap`, `core.autocrlf` CRLF checkout normalization, symlinks, gitlinks, and submodules;
- generated-file, minified-file, binary, UTF-16, and Git LFS filtering;
- deleted worktree files and per-file errors as warnings that do not stop the run.

It does not emulate `blame.ignoreRevsFile`, arbitrary clean/smudge filters or textconv drivers,
pathspec glob or magic syntax, partial-clone lazy fetching, SHA-256 or reftable repositories, or
every libgit2 repository format extension. Bare repositories are rejected with an error.
Non-UTF-8 paths and author names are handled lossily.

### Behavior notes

| Case | Behavior |
| :--- | :--- |
| Bare repository | error, exit 1 |
| CWD outside the worktree with `GIT_DIR`/`GIT_WORK_TREE` | analyzes the whole worktree |
| Whitespace-only uncommitted edits | attributes the line to `Not Committed Yet` |
| Non-ASCII path | preserves the path and may succeed where Git text handling would fail |
| Closed output pipe | exits successfully |
| Invoked from inside `.git` | finds no tracked files |
| Invalid UTF-8 path/author bytes | lossy conversion at display and JSON boundaries |
| libgit2 errors/warnings | libgit2 error text |

## Building from source

Development builds require Rust 1.88 or newer plus a C toolchain for vendored libgit2. Git is not
needed to run the resulting binary.

```console
cargo build --release
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Linux static builds use the musl target:

```console
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

Release binaries are built by cargo-dist and use the version declared in `Cargo.toml`.

## License

MIT. See [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
