# Third-party notices

`blamr` is distributed under the MIT License. Release binaries include the following
third-party components. The full license texts are available in the corresponding upstream
crates and repositories linked below.

## Runtime dependencies

| Component | Version used | License |
| :--- | :--- | :--- |
| [git2](https://github.com/rust-lang/git2-rs) | 0.21.0 | MIT OR Apache-2.0 |
| [libgit2-sys](https://github.com/rust-lang/git2-rs) | 0.18.8+1.9.7 | MIT OR Apache-2.0 |
| [libgit2](https://github.com/libgit2/libgit2) (vendored) | 1.9.7 | GPL-2.0 with a linking exception; see `COPYING` in libgit2 |
| [libz-sys](https://github.com/rust-lang/libz-sys) | 1.1.30 | MIT OR Apache-2.0 |
| [zlib](https://zlib.net/) (system or bundled) | 1.3.2 when bundled | Zlib |
| [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam) | 0.5.17 | MIT OR Apache-2.0 |
| [crossbeam-utils](https://github.com/crossbeam-rs/crossbeam) | 0.8.23 | MIT OR Apache-2.0 |
| [serde](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 |
| [serde_core](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 |
| [serde_json](https://github.com/serde-rs/json) | 1.0.154 | MIT OR Apache-2.0 |
| [itoa](https://github.com/dtolnay/itoa) | 1.0.18 | MIT OR Apache-2.0 |
| [memchr](https://github.com/BurntSushi/memchr) | 2.8.3 | Unlicense OR MIT |
| [zmij](https://github.com/dtolnay/zmij) | 1.0.23 | MIT |
| [bitflags](https://github.com/bitflags/bitflags) | 2.13.2 | MIT OR Apache-2.0 |
| [libc](https://github.com/rust-lang/libc) | 0.2.191 | MIT OR Apache-2.0 |
| [log](https://github.com/rust-lang/log) | 0.4.34 | MIT OR Apache-2.0 |

`libz-sys` links the platform zlib when available and otherwise builds bundled zlib 1.3.2.
`libgit2` is linked with its linking exception. This project does not modify or redistribute
libgit2 separately; the exception permits distribution of the combined work under the MIT
License. Refer to the upstream `COPYING` file for the authoritative terms.

## Development-only dependencies

The test suite also uses `assert_cmd`, `tempfile`, and `pretty_assertions` (MIT OR
Apache-2.0), together with their transitive dependencies. These are not included in release
archives.
