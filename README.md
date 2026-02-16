# ForgeFlow ⚡

> Terminal-first Git Forge workflow automation

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

ForgeFlow is a high-performance CLI tool for automating Git forge workflows. Browse repositories, fetch resources, and submit changes—all without leaving your terminal. Built with Rust for speed and reliability.

Originally designed for virtualization and kernel development teams, ForgeFlow works with any Git forge (GitCode, GitLab, GitHub) and streamlines the entire contribution pipeline.

---

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Usage](#usage)
- [Supported Providers](#supported-providers)
- [Development](#development)
- [License](#license)

---

## Features

- **⚡ Blazing Fast** — Rust-powered with minimal startup time and memory footprint
- **📂 Remote Browsing** — Explore repository contents without cloning
- **📥 One-Command Fetch** — Download files or entire directories instantly
- **🚀 Atomic Submissions** — Branch, commit, and create PRs in a single operation
- **📝 Audit-Ready Commits** — Automatic Signed-off-by trailers, trace metadata, and compliance formatting
- **🔧 Multi-Provider** — Works with GitCode, GitLab, GitHub, and compatible forges
- **🔒 Secure by Default** — Token-based auth, no credential storage in shell history

---

## Installation

### From Source

```bash
git clone https://github.com/nashzhou/forgeflow.git
cd forgeflow
cargo install --path .
```

### Prerequisites

- Rust 1.75+ (Edition 2024)
- Git forge account with API access

---

## Quick Start

### 1. Configure

Create your configuration file:

```bash
mkdir -p ~/.config/vkt
```

### 2. Browse

```bash
# List repository root
vkt list

# Explore specific directories
vkt list scripts/qemu/
vkt list configs/ --recursive
```

### 3. Fetch

```bash
# Download a file
vkt get scripts/debug.sh -o ./local-scripts/

# Download an entire directory
vkt get templates/ --output ./templates/
```

### 4. Submit

```bash
# Submit a file with automated PR creation
vkt submit ./my-script.sh \
    --target scripts/tools/ \
    --msg "feat: add debugging utility for virtio devices"
```

---

## Configuration

Configuration is stored in TOML format at `~/.config/vkt/config.toml`.

### Full Configuration Reference

```toml
[user]
name = "Developer Name"           # Required: Git commit author name
email = "dev@company.com"         # Required: Git commit author email
auto_signoff = true               # Optional: Add Signed-off-by automatically

[remote]
provider = "Gitcode"              # Required: GitCode, GitLab, or GitHub
api_url = "https://api.gitcode.com/api/v5"  # Required: API endpoint URL
token = "xxxxxxxxxxxx"        # Required: Personal access token

[repo]
project_id = "owner/repo"         # Required: Project identifier (owner/repo)
default_branch = "main"           # Optional: Base branch for PRs (default: main)

[template]
pr_prefix = "[VIRT-TOOL]"         # Optional: Prefix for PR titles
```

### Environment Variables

All configuration values can be overridden via environment variables:

```bash
export VKT_USER_NAME="Override Name"
export VKT_USER_EMAIL="override@example.com"
export VKT_REMOTE_TOKEN="new-token"
export VKT_REPO_PROJECT_ID="different/project"
```

### Provider-Specific Notes

#### GitCode

```toml
[remote]
provider = "Gitcode"
api_url = "https://api.gitcode.com/api/v5"
token = "xxxxxxxxxxxxxxxxxxxx"
```

---

## Usage

### `list` — Browse Remote Repository

List contents of a remote directory without cloning.

```bash
vkt list [PATH] [OPTIONS]
```

**Options:**

- `-r, --recursive` — Recursively list subdirectories

**Examples:**

```bash
# List root directory
vkt list

# List specific path
vkt list scripts/

# Recursive listing
vkt list tools/ --recursive
```

**Sample Output:**

```bash
.assets/
exercises/
.gitignore
Cargo.toml
README.md
build.rs
info.toml
```

---

### `get` — Download Files

Fetch files or directories from the remote repository.

```bash
vkt get <REMOTE_PATH> [OPTIONS]
```

**Options:**

- `-o, --output <PATH>` — Local destination path (default: current directory)
- `-f, --force` — Overwrite existing files without prompting

**Examples:**

```bash
# Fetch single file to current directory
vkt get configs/kernel-debug.config

# Fetch to specific location
vkt get scripts/qemu-launch.sh -o ~/bin/

# Fetch entire directory
vkt get templates/ -o ./my-templates/ --force
```

---

### `submit` — Submit Changes (Atomic Workflow)

The flagship feature: submit local files, create a branch, commit with trace metadata, and open a PR—all in one command.

```bash
vkt submit <LOCAL_PATH> --target <REMOTE_DIR> --msg <MESSAGE> [OPTIONS]
```

**Options:**

- `-t, --target <DIR>` — Target directory in remote repository (required)
- `-m, --msg <MESSAGE>` — Commit/PR message (required)
- `-b, --branch <NAME>` — Custom branch name (auto-generated if omitted)
- `-f, --force` — Skip confirmation prompts
- `--dry-run` — Preview actions without executing

**Examples:**

```bash
# Basic submission
vkt submit ./debug.sh \
    --target scripts/tools/ \
    --msg "feat: add virtio debugging script"

# With custom branch name
vkt submit ./config updates/ \
    --target configs/kernel/ \
    --msg "chore: update kernel configs for v6.8" \
    --branch feat/kernel-configs-v6.8

# Dry run to preview changes
vkt submit ./new-feature.sh \
    --target scripts/ \
    --msg "feat: implement new workflow" \
    --dry-run
```

**What Happens Behind the Scenes:**

1. **Conflict Check** — Verifies no file exists at the target path
2. **Branch Creation** — Creates feature branch from default branch
3. **Content Upload** — Uploads file(s) via API
4. **Commit Generation** — Creates commit with:
   - Your configured author info
   - Signed-off-by trailer (if `auto_signoff = true`)
   - Content hash for traceability
   - Timestamp and metadata
5. **PR Creation** — Opens merge request with:
   - Prefixed title (from config)
   - Auto-assigned reviewers
   - Link to uploaded content

**Sample Output:**

```bash
✅ File uploaded: scripts/tools/debug.sh (4.2 KB)
✅ Branch created: feat/add-virtio-debugging-script
✅ Commit: 8a3f2d1 — feat: add virtio debugging script
✅ PR #42 created: "[TEAM] feat: add virtio debugging script"
🔗 https://gitcode.com/owner/repo/pull/42
```

---

## Supported Providers

| Provider |  Status | API Version |         Notes           |
|----------|---------|-------------|-------------------------|
| GitCode  | ✅ Full |     v5      | Primary target platform |
| GitLab   | ❌ None |     v4      |  Self-hosted supported  |

---

## Development

### Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run with logs
RUST_LOG=debug cargo run -- list
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_submit_command

# Test with coverage
cargo tarpaulin --out Html
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Type check (fast)
cargo check
```

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- Built with [clap](https://github.com/clap-rs/clap) for elegant CLI parsing
- HTTP client powered by [reqwest](https://github.com/seanmonstar/reqwest)
- Async runtime via [tokio](https://github.com/tokio-rs/tokio)

---

<div align="center">

**[⬆ Back to Top](#forgeflow-)**

Made with 🦀 in Rust

</div>
