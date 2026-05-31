# ACLI — Atlassian CLI (Rust Port)

A synchronous, dependency-light command-line interface for Atlassian Cloud products — **Jira**, **Confluence**, **Bitbucket**, and **JSM Alerts** — ported from Go to Rust.

> **Based on [chinmaymk/acli](https://github.com/chinmaymk/acli)** — the original Go implementation.
> This Rust port preserves the same command structure and profiles while replacing the runtime with a simpler, dependency-free stack.

[![CI](https://github.com/neuro-ng/acli/actions/workflows/ci.yml/badge.svg)](https://github.com/neuro-ng/acli/actions/workflows/ci.yml)
[![Release](https://github.com/neuro-ng/acli/actions/workflows/release.yml/badge.svg)](https://github.com/neuro-ng/acli/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](LICENSE)

---

## Why Rust?

The Go version uses `cobra` and goroutines. This port follows Rich Hickey's **Simple Made Easy** — it removes every layer that isn't load-bearing:

| Concern | Choice | Why |
|---------|--------|-----|
| HTTP | `ureq` (blocking) | No async runtime, no thread pool, predictable stack |
| CLI parsing | Manual `match` chains | No macro magic, easy to trace |
| Auth | Hand-rolled Base64 | Zero deps, auditable |
| Multipart upload | Manual boundary | No third-party crate needed |
| ADF / XHTML render | Recursive descent | Tiny, no regex crates |

---

## Installation

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/neuro-ng/acli/main/install.sh | sh
```

Override version or install directory:

```sh
ACLI_VERSION=v0.1.0 ACLI_INSTALL_DIR=~/.local/bin \
  curl -fsSL https://raw.githubusercontent.com/neuro-ng/acli/main/install.sh | sh
```

### Windows (PowerShell)

Download `acli-<version>-x86_64-pc-windows-msvc.zip` from [GitHub Releases](https://github.com/neuro-ng/acli/releases), extract, and add to `$PATH`.

### From source

```sh
cargo build --release
# binary: target/release/acli-rust
```

---

## Configuration

Profiles are stored in `~/.config/acli/config.json`. Each holds your Atlassian URL, email, and API token.

```sh
acli config setup              # create/update the 'default' profile interactively
acli config setup work         # create a named profile
acli config list               # list all profiles
acli config show [name]        # print details (token masked)
acli config set-default <name> # change the default profile
acli config delete <name>      # remove a profile
```

Use `-p <name>` to select a non-default profile for any command.
Get your API token at <https://id.atlassian.com/manage-profile/security/api-tokens>.

---

## Global Flags

```
-p, --profile <name>   Profile to use (overrides default)
-o, --output  <fmt>    text (default) | json
```

---

## Jira

### Issues

```sh
acli jira issue list                           # recent issues (last 30d)
acli jira issue list --jql "project = ACLI"    # raw JQL
acli jira issue list --project ACLI --status "In Progress"
acli jira issue get   <key>
acli jira issue create --project ACLI --summary "Fix login" --type Bug
acli jira issue edit  <key> --summary "New title" --priority High
acli jira issue delete <key>
acli jira issue assign <key> <account-id>      # 'none' to unassign
acli jira issue transition <key> --status Done
acli jira issue transitions <key>              # list available transitions
acli jira issue attach <key> <file>
```

### Comments

```sh
acli jira issue comment list <key>
acli jira issue comment add  <key> --body "LGTM"
acli jira issue comment delete <key> <comment-id>
```

### Worklogs

```sh
acli jira issue worklog list <key>
acli jira issue worklog add  <key> --time-spent 2h [--comment "Debugging"]
acli jira issue worklog delete <key> <worklog-id>
```

### Agile

```sh
acli jira board list [--project ACLI]
acli jira board sprints <board-id>
acli jira sprint issues <sprint-id>
acli jira epic   issues <epic-key>
```

---

## Confluence

Page content is rendered from XHTML storage format to plain text (`<h1>` → `# Heading`, ordered/unordered lists, HTML entities, etc.).

### Spaces

```sh
acli conf space list
acli conf space get    <id>
acli conf space create --name "Team Docs" --key TEAM
acli conf space pages  <id> [--title "Onboarding"]
```

### Pages

```sh
acli conf page list [--space-id <id>] [--title "Getting Started"]
acli conf page get   <id>                   # add --body to fetch rendered content
acli conf page create --space-id <id> --title "Hello" --body "<p>Hi</p>"
acli conf page update <id> --title "Hello v2" --version 2
acli conf page delete <id>
```

---

## Bitbucket

### Repositories

```sh
acli bb repo list   <workspace>
acli bb repo get    <workspace> <slug>
acli bb repo create <workspace> <slug> --name "my-repo" [--public]
acli bb repo delete <workspace> <slug>
```

### Pull Requests

```sh
acli bb pr list    <workspace> <slug> [--state OPEN]
acli bb pr get     <workspace> <slug> <id>
acli bb pr create  <workspace> <slug> --title "Fix login" --source feature/login --dest main
acli bb pr approve <workspace> <slug> <id>
acli bb pr merge   <workspace> <slug> <id> [--strategy squash]
acli bb pr decline <workspace> <slug> <id>
```

### Pipelines

```sh
acli bb pipeline list  <workspace> <slug>
acli bb pipeline get   <workspace> <slug> <uuid>
acli bb pipeline run   <workspace> <slug> --branch main
acli bb pipeline stop  <workspace> <slug> <uuid>
acli bb pipeline steps <workspace> <slug> <uuid>
acli bb pipeline log   <workspace> <slug> <pipeline-uuid> <step-uuid>
```

---

## JSM Alerts (Opsgenie)

Cloud ID is resolved automatically from `/_edge/tenant_info` — no manual config needed.

```sh
acli alert list
acli alert list --status acknowledged
acli alert get    <id>
acli alert create "DB replica lag > 30s" [--priority P2] [--alias prod-db-lag]
acli alert ack    <id> [--note "Paging oncall"]
acli alert close  <id>
```

---

## Stdin Pipe (`-`)

Pass any command as a JSON array via stdin — useful for automation pipelines:

```sh
echo '["jira", "issue", "get", "ACLI-42"]' | acli -
```

---

## Testing

**69 tests — 24 unit + 45 integration.** Integration tests run against in-process TCP mock servers; no real Atlassian credentials needed.

```sh
cargo test
```

Tests are split by module, sharing helpers through a common module:

```
tests/
├── common/
│   └── mod.rs         # mock_profile, http_ok/201/202/204, start_mock_*
├── jira.rs            # issue, comment, worklog tests
├── agile.rs           # board, sprint, epic tests
├── alerts.rs          # JSM Ops alert tests
├── confluence.rs      # space and page tests (owns its mock server)
└── bitbucket.rs       # repo, PR, pipeline tests (owns its mock server)
```

Static analysis:

```sh
cargo fmt -- --check
cargo clippy -- -D warnings
```

---

## Project Structure

```
src/
├── main.rs        # CLI routing — explicit match chains, zero framework
├── lib.rs         # module re-exports
├── client.rs      # HTTP client: ureq, Basic/Bearer auth, multipart upload,
│                  #   hand-rolled Base64, JSM Cloud ID resolution
├── config.rs      # profile manager (~/.config/acli/config.json)
├── jira.rs        # Jira REST API + ADF ↔ plain-text renderer + unit tests
├── agile.rs       # Jira Agile REST API (boards, sprints, epics)
├── confluence.rs  # Confluence v2 API + XHTML storage → plain-text renderer
│                  #   + unit tests
├── bitbucket.rs   # Bitbucket Cloud REST API (repos, PRs, pipelines, logs)
└── alerts.rs      # JSM Ops (Opsgenie) REST API
```

---

## CI / CD

| Workflow | Trigger | What it does |
|----------|---------|--------------|
| `ci.yml` | push / PR to `main` | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` on Ubuntu + macOS + Windows |
| `release.yml` | push tag `v*.*.*` | builds 5 targets, packages archives, uploads to GitHub Release |

Cross-compilation targets:

- `x86_64-unknown-linux-musl` (static, portable)
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin` (Apple Silicon)
- `x86_64-pc-windows-msvc`

---

## License

Apache 2.0 — see [LICENSE](LICENSE).

Original Go implementation: [neuro-ng/acli](https://github.com/neuro-ng/acli)
