# blamr

> Ultra-fast, concurrent `git-blame` author attribution and line statistics CLI tool written in pure Go (zero CGO).

[![Go Report Card](https://goreportcard.com/badge/github.com/ajmalbuv/blamr)](https://goreportcard.com/report/github.com/ajmalbuv/blamr)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## ⚡ Highlights

- **Concurrent Execution**: Spawns a worker goroutine pool across all CPU cores for fast attribution even in large codebases.
- **Dual Invocation**: Seamlessly run as a standalone CLI (`blamr`) or as a native Git subcommand (`git blamr`).
- **SIMD Binary & Noise Filtering**: High-speed stack-allocated 8KB inspection with SIMD null-byte detection (`bytes.IndexByte`), UTF-16 BOM recognition, Git LFS pointer skipping, and automatic filtering of machine-generated lockfiles (`package-lock.json`, `bun.lock`, `go.sum`, etc.) and minified bundles.
- **Whitespace Churn Resilient**: Passes `-w` to Git blame to prevent mass reformatting, indentation fixes, and lint cleanups from distorting authorship statistics.
- **Terminal & CI Polish**: Automatic TTY detection suppresses carriage returns when output is redirected to files or run in CI environments, paired with ANSI `\033[K` clear-line sequences to avoid terminal ghosting.
- **Automation Ready**: Output structured, machine-readable JSON with `-json` for direct integration into CI/CD pipelines, PR bots, and team dashboards.
- **Attribution Precision**: Group by email (`-email`), filter blank/whitespace lines (`-ignore-blank`), or exclude uncommitted working tree changes (`-committed-only`).

---

## 🚀 Installation

### Via `go install`

```bash
# Install both standalone blamr and git-blamr
go install github.com/ajmalbuv/blamr/cmd/blamr@latest
go install github.com/ajmalbuv/blamr/cmd/git-blamr@latest
```

### Precompiled Binaries

Download static zero-dependency binaries for Windows, macOS, or Linux from [GitHub Releases](https://github.com/ajmalbuv/blamr/releases).

---

## 📖 Usage

### Basic Attribution

Run inside any Git repository to analyze all tracked text files:

```bash
blamr
# or natively via git
git blamr
```

**Output**:

```text
─────────────────────────────────────────────
Author                       Lines        %
─────────────────────────────────────────────
Alice Smith                   8200    65.9%  █████████████
Bob Jones                     4250    34.1%  ██████
─────────────────────────────────────────────
Total                        12450
```

### Scoping to Subdirectories & Modules

Target specific paths or packages:

```bash
blamr ./src ./internal
blamr frontend/
```

### Filtering by Extension

Only analyze specific file types:

```bash
blamr -ext=.go,.ts,.typ
```

### Precision Attribution Flags

```bash
# Group contributors by author email instead of author name
blamr -email

# Exclude empty and whitespace-only lines from the line count
blamr -ignore-blank

# Exclude uncommitted local working tree changes ("Not Committed Yet")
blamr -committed-only
```

### Structured JSON Output

Pipe directly to `jq`, dashboards, or CI steps:

```bash
blamr -json
```

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

---

## ⚙️ Flags Reference

| Flag              | Default    | Description                                                                         |
| :---------------- | :--------- | :---------------------------------------------------------------------------------- |
| `-ext <list>`     | `""`       | Comma-separated extensions to include (e.g. `.go,.ts`). Defaults to all text files. |
| `-j <num>`        | `NumCPU()` | Number of concurrent `git-blame` worker goroutines.                                 |
| `-email`          | `false`    | Aggregate lines by author email address instead of name.                            |
| `-ignore-blank`   | `false`    | Ignore empty and whitespace-only lines.                                             |
| `-committed-only` | `false`    | Exclude uncommitted working tree changes (`Not Committed Yet`).                     |
| `-json`           | `false`    | Output results in structured JSON format.                                           |
| `-v`, `-version`  | `false`    | Print version, OS, architecture, and exit.                                          |
| `-h`, `-help`     | `false`    | Display usage instructions and flag reference.                                      |

---

## 🏗️ Architecture

```text
blamr/
├── cmd/
│   ├── blamr/
│   │   └── main.go             # Standalone CLI entrypoint
│   └── git-blamr/
│       └── main.go             # Git subcommand entrypoint
├── internal/
│   ├── app/
│   │   └── app.go              # Shared CLI coordinator
│   ├── config/
│   │   ├── flags.go            # Flag definitions, validation, and Config struct
│   │   └── flags_test.go       # Configuration & CLI parsing unit tests
│   ├── filter/
│   │   ├── binary.go           # SIMD null-byte check, UTF-16 BOM, LFS detection
│   │   ├── generated.go        # Lockfile & minified asset skips, extension matcher
│   │   ├── filter.go           # Unified file inspection
│   │   └── filter_test.go      # In-memory unit tests
│   ├── blame/
│   │   ├── runner.go           # Worker pool & git blame execution pipeline
│   │   ├── parser.go           # Line-porcelain parser (email, blank & uncommitted filters)
│   │   └── parser_test.go      # Synthetic porcelain unit tests
│   └── output/
│       ├── terminal.go         # TTY detection, ANSI clear line (\033[K), table renderer
│       ├── json.go             # Structured JSON serializer
│       └── output_test.go      # Terminal & JSON formatting tests
├── .goreleaser.yaml            # Multi-arch automated release pipeline
├── Justfile                    # Build, test, and release recipes
├── go.mod
├── todo.md                     # Roadmap checklist
└── README.md
```

---

## 🛠️ Development

Requirements: [Go 1.22+](https://go.dev) and optionally [Just](https://github.com/casey/just).

```bash
# Build blamr binary
just build

# Build both blamr and git-blamr binaries
just build-all

# Run all unit tests
just test

# Clean artifacts
just clean
```

---

## 📄 License

MIT © [Ajmal Basheer](https://github.com/ajmalbuv)
