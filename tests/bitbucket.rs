mod common;

use std::io::Write;
use std::net::TcpListener;
use std::thread;

use acli_rust::bitbucket;
use acli_rust::client::Client;
use common::{http_201, http_204, http_ok, mock_profile, read_request};

/// Bitbucket API lives at api.bitbucket.org/2.0, so the mock server must
/// handle absolute URLs that the client builds. We expose it on localhost and
/// the Client routes full-URL paths straight through.
pub fn start_mock_bitbucket_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };

            let req = read_request(&mut stream);

            // Match most-specific paths first to avoid substring collisions.
            let response =
                // --- PRs (most specific sub-actions first) ---
                if req.contains("/pullrequests/42/approve") {
                    http_200_empty()
                } else if req.contains("/pullrequests/42/merge") {
                    http_ok(r#"{"id":42,"title":"Fix bug","state":"MERGED","author":{"display_name":"Dev"},"source":{"branch":{"name":"fix/bug"}},"destination":{"branch":{"name":"main"}}}"#)
                } else if req.contains("/pullrequests/42/decline") {
                    http_ok(r#"{"id":42,"title":"Fix bug","state":"DECLINED","author":{"display_name":"Dev"},"source":{"branch":{"name":"fix/bug"}},"destination":{"branch":{"name":"main"}}}"#)
                } else if req.contains("GET /2.0/repositories/myteam/acli/pullrequests/42") {
                    http_ok(r#"{"id":42,"title":"Fix bug","state":"OPEN","author":{"display_name":"Dev","uuid":"{123}"},"source":{"branch":{"name":"fix/bug"}},"destination":{"branch":{"name":"main"}},"description":"Fixes the critical bug","created_on":"2026-05-30T00:00:00Z","updated_on":"2026-05-31T00:00:00Z"}"#)
                } else if req.contains("POST /2.0/repositories/myteam/acli/pullrequests") {
                    http_201(r#"{"id":43,"title":"Add feature","state":"OPEN","author":{"display_name":"Dev"},"source":{"branch":{"name":"feature/new"}},"destination":{"branch":{"name":"main"}}}"#)
                } else if req.contains("GET /2.0/repositories/myteam/acli/pullrequests") {
                    http_ok(r#"{"values":[{"id":42,"title":"Fix bug","state":"OPEN","author":{"display_name":"Dev"},"source":{"branch":{"name":"fix/bug"}},"destination":{"branch":{"name":"main"}}}],"size":1}"#)
                // --- Pipelines (deepest paths first) ---
                } else if req.contains("/steps/step-uuid-1/log") {
                    http_ok("Build started\nCompiling...\nBuild succeeded\n")
                } else if req.contains("/pipelines/pipe-uuid-1/steps/") {
                    http_ok(r#"{"values":[{"uuid":"step-uuid-1","name":"Build","state":{"name":"COMPLETED","result":{"name":"SUCCESSFUL"}},"duration_in_seconds":45}]}"#)
                } else if req.contains("/pipelines/pipe-uuid-1/stopPipeline") {
                    http_204()
                } else if req.contains("/pipelines/pipe-uuid-1") {
                    http_ok(r#"{"uuid":"pipe-uuid-1","build_number":7,"state":{"name":"COMPLETED","result":{"name":"SUCCESSFUL"}},"trigger":{"name":"MANUAL"},"target":{"ref_name":"main","commit":{"hash":"abc123"}},"created_on":"2026-05-31T00:00:00Z","completed_on":"2026-05-31T00:01:00Z","duration_in_seconds":60}"#)
                } else if req.contains("POST /2.0/repositories/myteam/acli/pipelines/") {
                    http_201(r#"{"uuid":"pipe-uuid-2","build_number":8,"state":{"name":"PENDING"},"trigger":{"name":"MANUAL"},"target":{"ref_name":"feature/new"},"created_on":"2026-05-31T00:02:00Z"}"#)
                } else if req.contains("GET /2.0/repositories/myteam/acli/pipelines/") {
                    http_ok(r#"{"values":[{"uuid":"pipe-uuid-1","build_number":7,"state":{"name":"COMPLETED","result":{"name":"SUCCESSFUL"}},"trigger":{"name":"PUSH"},"target":{"ref_name":"main"},"created_on":"2026-05-31T00:00:00Z"}],"size":1}"#)
                // --- Repos (single repo last so sub-paths don't collide) ---
                } else if req.contains("POST /2.0/repositories/myteam/new-repo") {
                    http_201(r#"{"full_name":"myteam/new-repo","slug":"new-repo","name":"new-repo","is_private":true,"language":"rust","created_on":"2026-05-31T00:00:00Z","updated_on":"2026-05-31T00:00:00Z"}"#)
                } else if req.contains("DELETE /2.0/repositories/myteam/old-repo") {
                    http_204()
                } else if req.contains("GET /2.0/repositories/myteam/acli ") {
                    // space after slug ensures this doesn't match sub-paths
                    http_ok(r#"{"full_name":"myteam/acli","slug":"acli","name":"ACLI","is_private":true,"language":"rust","scm":"git","created_on":"2026-01-01T00:00:00Z","updated_on":"2026-05-31T00:00:00Z","mainbranch":{"name":"main"}}"#)
                } else if req.contains("GET /2.0/repositories/myteam") {
                    http_ok(r#"{"values":[{"full_name":"myteam/acli","slug":"acli","name":"ACLI","is_private":true,"language":"rust","updated_on":"2026-05-31T00:00:00Z"},{"full_name":"myteam/widget","slug":"widget","name":"Widget","is_private":false,"language":"go","updated_on":"2026-05-30T00:00:00Z"}],"size":2}"#)
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
                };

            let _ = stream.write_all(response.as_bytes());
        }
    });

    // Return the local base URL — the bitbucket module uses absolute URLs
    // starting with `https://api.bitbucket.org`, but Client::request passes
    // absolute URLs through unchanged, so we patch them to our local server
    // by setting the base_url to point here (unused for Bitbucket since
    // all paths are already absolute). We return the server address for
    // constructing test profiles.
    format!("http://127.0.0.1:{}", port)
}

fn http_200_empty() -> String {
    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_string()
}

/// Creates a test client pointing at the local mock server.
/// Bitbucket tests use relative `/2.0/...` paths prepended with base_url.
fn bb_client_at(base_url: &str) -> Client {
    Client::new(mock_profile(base_url))
}

fn bb_req(
    client: &Client,
    method: &str,
    path: &str,
    query: Option<&[(&str, &str)]>,
    body: Option<serde_json::Value>,
) -> Result<String, String> {
    client.request(method, path, query, body)
}

// ---------------------------------------------------------------------------
// Phase 4 tests — Bitbucket repos
// ---------------------------------------------------------------------------

#[test]
fn test_bb_list_repos() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    // Call using relative path (client prepends base_url)
    let resp = bb_req(
        &client,
        "GET",
        "/2.0/repositories/myteam",
        Some(&[("pagelen", "50")]),
        None,
    )
    .unwrap();
    let page: bitbucket::ReposPage = serde_json::from_str(&resp).unwrap();
    assert_eq!(page.values.len(), 2);
    assert_eq!(page.values[0].full_name, "myteam/acli");
}

#[test]
fn test_bb_get_repo() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let resp = bb_req(&client, "GET", "/2.0/repositories/myteam/acli", None, None).unwrap();
    let repo: bitbucket::Repo = serde_json::from_str(&resp).unwrap();
    assert_eq!(repo.full_name, "myteam/acli");
    assert_eq!(repo.main_branch.as_ref().unwrap().name, "main");
    assert!(repo.is_private);
}

#[test]
fn test_bb_create_repo() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let body = serde_json::json!({"scm":"git","name":"new-repo","is_private":true});
    let resp = bb_req(
        &client,
        "POST",
        "/2.0/repositories/myteam/new-repo",
        None,
        Some(body),
    )
    .unwrap();
    let repo: bitbucket::Repo = serde_json::from_str(&resp).unwrap();
    assert_eq!(repo.full_name, "myteam/new-repo");
}

#[test]
fn test_bb_delete_repo() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    bb_req(
        &client,
        "DELETE",
        "/2.0/repositories/myteam/old-repo",
        None,
        None,
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Phase 4 tests — Bitbucket PRs
// ---------------------------------------------------------------------------

#[test]
fn test_bb_list_prs() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let resp = bb_req(
        &client,
        "GET",
        "/2.0/repositories/myteam/acli/pullrequests",
        Some(&[("pagelen", "50")]),
        None,
    )
    .unwrap();
    let page: bitbucket::PrsPage = serde_json::from_str(&resp).unwrap();
    assert_eq!(page.values.len(), 1);
    assert_eq!(page.values[0].id, 42);
    assert_eq!(page.values[0].state, "OPEN");
}

#[test]
fn test_bb_get_pr() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let resp = bb_req(
        &client,
        "GET",
        "/2.0/repositories/myteam/acli/pullrequests/42",
        None,
        None,
    )
    .unwrap();
    let pr: bitbucket::PullRequest = serde_json::from_str(&resp).unwrap();
    assert_eq!(pr.id, 42);
    assert_eq!(pr.title, "Fix bug");
    assert_eq!(pr.source.branch.name, "fix/bug");
    assert_eq!(pr.destination.branch.name, "main");
}

#[test]
fn test_bb_create_pr() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let body = serde_json::json!({
        "title": "Add feature",
        "source": {"branch": {"name": "feature/new"}},
        "destination": {"branch": {"name": "main"}}
    });
    let resp = bb_req(
        &client,
        "POST",
        "/2.0/repositories/myteam/acli/pullrequests",
        None,
        Some(body),
    )
    .unwrap();
    let pr: bitbucket::PullRequest = serde_json::from_str(&resp).unwrap();
    assert_eq!(pr.id, 43);
    assert_eq!(pr.title, "Add feature");
}

#[test]
fn test_bb_approve_pr() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    bb_req(
        &client,
        "POST",
        "/2.0/repositories/myteam/acli/pullrequests/42/approve",
        None,
        None,
    )
    .unwrap();
}

#[test]
fn test_bb_merge_pr() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let resp = bb_req(
        &client,
        "POST",
        "/2.0/repositories/myteam/acli/pullrequests/42/merge",
        None,
        None,
    )
    .unwrap();
    let pr: bitbucket::PullRequest = serde_json::from_str(&resp).unwrap();
    assert_eq!(pr.state, "MERGED");
}

#[test]
fn test_bb_decline_pr() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let resp = bb_req(
        &client,
        "POST",
        "/2.0/repositories/myteam/acli/pullrequests/42/decline",
        None,
        None,
    )
    .unwrap();
    let pr: bitbucket::PullRequest = serde_json::from_str(&resp).unwrap();
    assert_eq!(pr.state, "DECLINED");
}

// ---------------------------------------------------------------------------
// Phase 4 tests — Bitbucket Pipelines
// ---------------------------------------------------------------------------

#[test]
fn test_bb_list_pipelines() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let resp = bb_req(
        &client,
        "GET",
        "/2.0/repositories/myteam/acli/pipelines/",
        Some(&[("pagelen", "25"), ("sort", "-created_on")]),
        None,
    )
    .unwrap();
    let page: bitbucket::PipelinesPage = serde_json::from_str(&resp).unwrap();
    assert_eq!(page.values.len(), 1);
    assert_eq!(page.values[0].build_number, 7);
}

#[test]
fn test_bb_get_pipeline() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let resp = bb_req(
        &client,
        "GET",
        "/2.0/repositories/myteam/acli/pipelines/pipe-uuid-1",
        None,
        None,
    )
    .unwrap();
    let p: bitbucket::Pipeline = serde_json::from_str(&resp).unwrap();
    assert_eq!(p.build_number, 7);
    assert_eq!(p.state.result.as_ref().unwrap().name, "SUCCESSFUL");
    assert_eq!(p.target.ref_name.as_deref(), Some("main"));
}

#[test]
fn test_bb_run_pipeline() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let body = serde_json::json!({
        "target": {"ref_type": "branch", "type": "pipeline_ref_target", "ref_name": "feature/new"}
    });
    let resp = bb_req(
        &client,
        "POST",
        "/2.0/repositories/myteam/acli/pipelines/",
        None,
        Some(body),
    )
    .unwrap();
    let p: bitbucket::Pipeline = serde_json::from_str(&resp).unwrap();
    assert_eq!(p.build_number, 8);
}

#[test]
fn test_bb_stop_pipeline() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    bb_req(
        &client,
        "POST",
        "/2.0/repositories/myteam/acli/pipelines/pipe-uuid-1/stopPipeline",
        None,
        None,
    )
    .unwrap();
}

#[test]
fn test_bb_list_pipeline_steps() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let resp = bb_req(
        &client,
        "GET",
        "/2.0/repositories/myteam/acli/pipelines/pipe-uuid-1/steps/",
        None,
        None,
    )
    .unwrap();
    let steps: bitbucket::StepsPage = serde_json::from_str(&resp).unwrap();
    assert_eq!(steps.values.len(), 1);
    assert_eq!(steps.values[0].name.as_deref(), Some("Build"));
    assert_eq!(
        steps.values[0].state.result.as_ref().unwrap().name,
        "SUCCESSFUL"
    );
}

#[test]
fn test_bb_get_pipeline_log() {
    let url = start_mock_bitbucket_server();
    let client = bb_client_at(&url);

    let log = bb_req(
        &client,
        "GET",
        "/2.0/repositories/myteam/acli/pipelines/pipe-uuid-1/steps/step-uuid-1/log",
        None,
        None,
    )
    .unwrap();
    assert!(log.contains("Build started"));
    assert!(log.contains("Build succeeded"));
}
