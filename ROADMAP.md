# ACLI (Rust) Migration Roadmap

This document outlines the migration plan of the Atlassian CLI (`acli`) from Go to Rust. It defines our architectural philosophy, command mapping, and phase breakdown.

## Status Legend

| Icon | Meaning |
|------|---------|
| ✅ | Complete — implemented and tested |
| 🔧 | In Progress — partially implemented |
| 🔲 | Outstanding — not yet started |

## Core Philosophy

Our migration strictly follows the principles in [SIMPLE_MINDSET_GUIDE.md](file:///home/neu/workspace/acli/SIMPLE_MINDSET_GUIDE.md), adapting Rich Hickey's "Simple Made Easy" philosophy to the codebase:

1.  **Simplicity Over Ease:** Avoid "magical" frameworks that hide complexity (like heavy macro-based CLI parsers). Write explicit, readable Rust.
2.  **No Async Runtime (De-complecting IO):** Go leverages goroutines natively, but Rust requires a runtime (e.g. Tokio) for async. To keep execution simple and keep CLI control flow synchronous, we use blocking HTTP requests with `ureq`. This decouples standard network communication from complex thread pools and async state machines.
3.  **Strict Error Handling:** Use explicit, domain-specific `Result` and `Error` types. No generic panics.
4.  **Composition:** Keep the generic HTTP client (`Client`) completely separate from product API clients (`JiraClient`, `AlertClient`).
5.  **Separation of Concerns:** Separate API payloads (JSON definitions) from presentation layers (terminal rendering, table formatting).

---

## Migration Phases

### Phase 1: Core Infrastructure & Alerts — ✅ Complete

| Item | Status | File(s) |
|------|--------|---------|
| Initialize Cargo workspace | ✅ | `Cargo.toml` |
| Profile configuration (`~/.config/acli/config.json`) | ✅ | `src/config.rs` |
| Generic HTTP client (Basic/Bearer auth, `ureq`) | ✅ | `src/client.rs` |
| Cloud ID resolution via `/_edge/tenant_info` | ✅ | `src/client.rs` |
| Jira issue search (`jira issue list`, JQL) | ✅ | `src/jira.rs` |
| Jira issue get (`jira issue get <key>`) | ✅ | `src/jira.rs` |
| ADF → plain text renderer | ✅ | `src/jira.rs` |
| JSM Alerts — list, get, create, ack, close | ✅ | `src/alerts.rs` |
| CLI routing for config, jira, alert commands | ✅ | `src/main.rs` |
| Stdin JSON argument redirect (`-`) | ✅ | `src/main.rs` |
| Integration tests (mock server) | ✅ | `tests/integration_test.rs` |

---

### Phase 2: Full Jira Support — ✅ Complete

#### Issue Write Operations (Data Layer)

| Item | Status | File(s) |
|------|--------|---------|
| `create_issue()` — POST `/rest/api/3/issue` | ✅ | `src/jira.rs` |
| `edit_issue()` — PUT `/rest/api/3/issue/{key}` | ✅ | `src/jira.rs` |
| `delete_issue()` — DELETE `/rest/api/3/issue/{key}` | ✅ | `src/jira.rs` |
| `assign_issue()` — PUT `/rest/api/3/issue/{key}/assignee` | ✅ | `src/jira.rs` |
| `text_to_adf()` — plain text → ADF conversion | ✅ | `src/jira.rs` |
| `get_transitions()` / `do_transition()` | ✅ | `src/jira.rs` |

#### Comments & Worklogs (Data Layer)

| Item | Status | File(s) |
|------|--------|---------|
| `list_comments()` / `add_comment()` / `delete_comment()` | ✅ | `src/jira.rs` |
| `list_worklogs()` / `add_worklog()` / `delete_worklog()` | ✅ | `src/jira.rs` |

#### Attachments

| Item | Status | File(s) |
|------|--------|---------|
| `attach_file()` — data function | ✅ | `src/jira.rs` |
| `request_multipart()` — multipart form upload in client | ✅ | `src/client.rs` |

#### Agile (Board, Sprint, Epic)

| Item | Status | File(s) |
|------|--------|---------|
| `agile.rs` module (boards, sprints, epics) | ✅ | `src/agile.rs` |

#### CLI Routing for Phase 2 Commands

| Item | Status | File(s) |
|------|--------|---------|
| `jira issue create/edit/delete/assign/transition` routing | ✅ | `src/main.rs` |
| `jira comment list/add/delete` routing | ✅ | `src/main.rs` |
| `jira worklog list/add/delete` routing | ✅ | `src/main.rs` |
| `jira attach <key> <file>` routing | ✅ | `src/main.rs` |
| `jira board/sprint/epic` routing | ✅ | `src/main.rs` |
| Integration tests for M2 operations | ✅ | `tests/integration_test.rs` |

---

### Phase 3: Confluence Support — ✅ Complete

| Item | Status | File(s) |
|------|--------|---------|
| `confluence.rs` module — spaces (list, get, create, space pages) | ✅ | `src/confluence.rs` |
| `confluence.rs` module — pages (list, get, create, update, delete) | ✅ | `src/confluence.rs` |
| XHTML storage → plain text renderer | ✅ | `src/confluence.rs` |
| CLI routing for `confluence`/`conf`/`c` command | ✅ | `src/main.rs` |
| Integration tests for Confluence | ✅ | `tests/integration_test.rs` |

---

### Phase 4: Bitbucket Support — ✅ Complete

| Item | Status | File(s) |
|------|--------|---------|
| `bitbucket.rs` module — repos (list, get, create, delete) | ✅ | `src/bitbucket.rs` |
| `bitbucket.rs` module — PRs (list, get, create, approve, merge, decline) | ✅ | `src/bitbucket.rs` |
| `bitbucket.rs` module — pipelines (list, get, run, stop, steps, log) | ✅ | `src/bitbucket.rs` |
| CLI routing for `bitbucket`/`bb` command | ✅ | `src/main.rs` |
| Integration tests for Bitbucket | ✅ | `tests/integration_test.rs` |

---

### Phase 5: CI/CD & Distribution — ✅ Complete

| Item | Status | File(s) |
|------|--------|---------|
| Unit tests (base64, ADF renderer, XHTML renderer) | ✅ | `src/client.rs`, `src/jira.rs`, `src/confluence.rs` |
| `cargo clippy` / `cargo fmt` clean | ✅ | — |
| CI workflow (fmt + clippy + test on push/PR) | ✅ | `.github/workflows/ci.yml` |
| GitHub Actions release workflow | ✅ | `.github/workflows/release.yml` |
| Cross-compilation (Linux musl/gnu, macOS x64/arm64, Windows) | ✅ | `.github/workflows/release.yml` |
| Installer script (`install.sh`) | ✅ | `install.sh` |

---

## Service Manager Alerts Capabilities — ✅ Complete

A key enhancement in the Rust codebase is integration with the **Jira Service Management Operations (Opsgenie) REST API** to manage alerts.

### Subcommands
-   ✅ `acli alert list` — List active alerts.
-   ✅ `acli alert get <id>` — Fetch details of a specific alert.
-   ✅ `acli alert create <message>` — Create a new alert.
-   ✅ `acli alert acknowledge <id>` — Acknowledge an alert.
-   ✅ `acli alert close <id>` — Close an alert.

### Authentication & API Routing
-   **Authentication:** Basic Auth (`email` + `api_token`) is automatically handled.
-   **Cloud ID Resolution:** Unlike other REST endpoints that use the instance URL directly, JSM Ops APIs require a `cloudId` (`https://api.atlassian.com/jsm/ops/api/{cloudId}/v1/alerts`). The Rust client automatically fetches the cloudId from `<instance-url>/_edge/tenant_info` and handles it transparently, avoiding manual user configuration.
