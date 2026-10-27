# Plan: `blamr` Rust rewrite on `ajmalbuv/blamr`, release `v0.0.3`

Status: approved direction, execution deferred until follow-up.

## Confirmed decisions

- The Rust rewrite lands as **one new commit on top of the existing Go `main`**, not a force-push.
- Committed version: `0.0.3`; release tag: `v0.0.3`.
- `v0.0.2` remains visible as the final Go release.
- `aarch64-pc-windows-msvc` stays in the target matrix for parity with the Go release.
- `CARGO_REGISTRY_TOKEN` is the crate publishing secret. This is the cargo-dist / crates.io standard name; use a crates.io API token with publish-new and publish-update scopes.
- Distribution route: cargo-dist + crates.io publication + cargo-binstall.
- Do not tag or publish until the follow-up review.

## 1. Rename and clean the Rust project

- `Cargo.toml`:
  - `name = "blamr"`
  - `version = "0.0.3"`
  - `repository = "https://github.com/ajmalbuv/blamr"`
  - remove `publish = false`
  - binaries `blamr` and `git-blamr`
  - paths `src/bin/blamr.rs` and `src/bin/git-blamr.rs`
- Rename the two bin files and update `blamer::run()` to `blamr::run()`.
- Update all user-visible strings:
  - `src/lib.rs:29`
  - `src/config.rs:294-298`, `src/config.rs:427`
  - `src/blame.rs:43`
  - `src/filter.rs:152`
  - `tests/cli.rs:290,307,311,313,319` and all `cargo_bin("blamer")` calls
  - `tests/fixtures.rs:126,138`
- Drop the `BLAMER_VERSION` override in `src/output.rs:70`; use `CARGO_PKG_VERSION` directly because cargo-dist enforces tag == crate version.
- Delete `Cross.toml`; cargo-dist handles cross-compilation.
- Rewrite `README.md` and `THIRD_PARTY_NOTICES.md` for `blamr`; remove Go/CGO/`go install` claims and the "no crates.io / no cargo install" paragraph.
- Add `CHANGELOG.md` with a `0.0.3` Rust-rewrite entry; cargo-dist uses it for release notes.
- Add the `cargo` ecosystem to `.github/dependabot.yml`; it currently monitors only GitHub Actions.
- Delete the stale local GoReleaser `dist/` directory.
- Regenerate `Cargo.lock`.
- Verify: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.

## 2. Add cargo-dist

- Add `origin` pointing at `https://github.com/ajmalbuv/blamr.git`.
- `git fetch origin main --tags`.
- Install `cargo-dist` 0.32.0 and run `dist init -y`.
- Edit the generated config. Proposed `dist-workspace.toml`:

```toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "0.32.0"
ci = "github"
installers = ["shell", "powershell"]
targets = [
  "x86_64-pc-windows-msvc",
  "aarch64-pc-windows-msvc",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-unknown-linux-musl",
  "aarch64-unknown-linux-musl",
]
publish-jobs = ["./publish-crates"]
install-path = "CARGO_HOME"
unix-archive = ".tar.gz"
windows-archive = ".zip"
include = ["THIRD_PARTY_NOTICES.md"]
```

- Add explicit cargo-binstall metadata to `Cargo.toml` so Windows zip-root layout and tarball subdirectories are deterministic:

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/{ name }-{ target }.tar.gz"
bin-dir = "{ name }-{ target }/{ bin }{ binary-ext }"
pkg-fmt = "tgz"

[package.metadata.binstall.overrides.'cfg(windows)']
pkg-url = "{ repo }/releases/download/v{ version }/{ name }-{ target }.zip"
bin-dir = "{ bin }{ binary-ext }"
pkg-fmt = "zip"
```

- Run `dist generate`, `dist plan`, then `dist build --target x86_64-pc-windows-msvc`.
- Confirm the local archive contains both `blamr.exe` and `git-blamr.exe`.
- Commit the rename and dist setup locally on the Rust `main` as a backup checkpoint.

## 3. Create the rewrite commit on the Go history

- `git branch backup/rust-main main` to preserve the Rust history locally.
- `git switch -c rewrite origin/main`.
- `git read-tree -u --reset main` so the tree is exactly the renamed Rust project.
- `git add -A`.
- Commit as `Rewrite in Rust` with a body noting the rename and that `v0.0.2` is the final Go release.
- Verify the commit parent is the Go `main`, and that `go.mod`, `cmd/`, `internal/`, `.goreleaser.yaml`, and `Justfile` are gone.
- `git push origin rewrite:main` (fast-forward, no force).
- If branch protection blocks the push, use a PR or temporarily allow admin.
- Align local `main` to the rewrite commit; keep `backup/rust-main`.
- Wait for CI, actionlint, and audit to pass.

## 4. Prepare the release, then stop

- Add the `CARGO_REGISTRY_TOKEN` repository secret.
- Run `cargo publish --dry-run --locked`.
- Confirm `Cargo.toml` version is `0.0.3` and the tag will be `v0.0.3`.
- Stop for approval before tagging.

## 5. Tag and release

```powershell
git tag -a v0.0.3 -m "blamr v0.0.3"
git push origin v0.0.3
```

The dist workflow will:

- build all six targets
- produce archives, checksums, shell and PowerShell installers, and `dist-manifest.json`
- create the GitHub release
- publish `blamr 0.0.3` to crates.io

`v0.0.2` remains visible and unchanged as the last Go release.

## 6. Post-release verification

- GitHub release has both binaries in every archive and `-version` works.
- crates.io shows `blamr 0.0.3`.
- `cargo binstall blamr@0.0.3 --no-confirm` installs both binaries from prebuilt artifacts.
- `cargo install blamr --version 0.0.3` compiles where CMake and a C toolchain exist.
- `git blamr` works after installation.
- Update README with real installer URLs and badges.
- Label `v0.0.2` as the final Go release.

## Risks / checks

- First cargo-dist release must prove that vendored libgit2 builds for `aarch64-unknown-linux-musl` and `aarch64-pc-windows-msvc`.
- Branch protection may block a direct `main` push.
- `cargo install` remains slow because it compiles vendored libgit2; binstall and the installers are the promoted paths.
- Do not push tags or trigger the release workflow before the follow-up approval.
