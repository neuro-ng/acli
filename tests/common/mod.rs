#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use acli_rust::config::Profile;

pub fn mock_profile(base_url: &str) -> Profile {
    Profile {
        name: "test".to_string(),
        atlassian_url: base_url.to_string(),
        email: "test@example.com".to_string(),
        api_token: "test-token".to_string(),
        defaults: None,
    }
}

pub fn http_ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

pub fn http_201(body: &str) -> String {
    format!(
        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

pub fn http_202(body: &str) -> String {
    format!(
        "HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

pub fn http_204() -> String {
    "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n".to_string()
}

/// Reads a complete HTTP request (headers + body).
///
/// A single `read()` call may return only the headers on Windows, leaving the
/// body unread. Dropping the stream with unread data triggers a RST, which
/// Windows propagates back to the client as WSAECONNRESET (10054) — even if
/// the server already wrote a valid response. Reading the full request first
/// ensures the stream closes cleanly with a FIN on all platforms.
pub fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut raw: Vec<u8> = Vec::new();
    let mut buf = [0u8; 4096];

    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);

                // Locate the end of the HTTP header block (\r\n\r\n)
                if let Some(hdr_end) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    let body_start = hdr_end + 4;
                    // Honour Content-Length so we wait for the full body
                    let content_length = String::from_utf8_lossy(&raw[..hdr_end])
                        .split("\r\n")
                        .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                        .and_then(|l| l.splitn(2, ':').nth(1))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);

                    if raw.len() >= body_start + content_length {
                        break; // complete request received
                    }
                }

                if raw.len() > 16_384 {
                    break; // safety limit — no test request exceeds this
                }
            }
        }
    }

    String::from_utf8_lossy(&raw).to_string()
}

/// Starts the Phase 1 mock server (JSM alerts + basic Jira reads).
pub fn start_mock_atlassian_sdk() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };

            let req = read_request(&mut stream);

            let response = if req.contains("GET /_edge/tenant_info") {
                http_ok(r#"{"cloudId":"mock-cloud-id-123"}"#)
            } else if req.contains("GET /rest/api/3/issue/TEST-101 ")
                || req.contains("GET /rest/api/3/issue/TEST-101\r\n")
            {
                http_ok(
                    r#"{
                    "id": "101",
                    "key": "TEST-101",
                    "fields": {
                        "summary": "Implement local mock SDK",
                        "description": "Create a simple local mock server in Rust",
                        "issuetype": { "name": "Story" },
                        "status": { "name": "In Progress" },
                        "priority": { "name": "High" },
                        "assignee": { "displayName": "Rust Developer" },
                        "reporter": { "displayName": "System" },
                        "created": "2026-05-30T00:00:00Z",
                        "updated": "2026-05-30T00:05:00Z",
                        "labels": ["rust", "test"],
                        "components": [],
                        "fixVersions": []
                    }
                }"#,
                )
            } else if req.contains("GET /rest/api/3/search/jql") {
                http_ok(
                    r#"{
                    "startAt": 0, "maxResults": 50, "total": 1,
                    "issues": [{
                        "id": "101", "key": "TEST-101",
                        "fields": {
                            "summary": "Implement local mock SDK",
                            "description": null,
                            "issuetype": { "name": "Story" },
                            "status": { "name": "In Progress" },
                            "priority": { "name": "High" },
                            "assignee": { "displayName": "Rust Developer" },
                            "reporter": null, "created": null, "updated": null,
                            "labels": [], "components": [], "fixVersions": []
                        }
                    }]
                }"#,
                )
            } else if req.contains("GET /jsm/ops/api/mock-cloud-id-123/v1/alerts/") {
                // Single alert: real Opsgenie API returns the Alert object directly (no wrapper).
                http_ok(
                    r#"{"id":"alert-uuid-1","tinyId":"101","alias":"alert-alias-1","message":"Memory leak detected","status":"open","acknowledged":false,"createdAt":"2026-05-30T00:00:00Z","priority":"P2","tags":[],"responders":[],"actions":[]}"#,
                )
            } else if req.contains("GET /jsm/ops/api/mock-cloud-id-123/v1/alerts") {
                // List alerts: real JSM API wraps in `values`
                http_ok(
                    r#"{
                    "values": [{
                        "id": "alert-uuid-1", "tinyId": "101", "alias": "alert-alias-1",
                        "message": "Memory leak detected", "status": "open",
                        "acknowledged": false, "createdAt": "2026-05-30T00:00:00Z", "priority": "P2",
                        "tags": [], "responders": [], "actions": []
                    }],
                    "links": {"next": ""},
                    "count": 1
                }"#,
                )
            } else if req.contains("POST /jsm/ops/api/mock-cloud-id-123/v1/alerts") {
                if req.contains("/acknowledge") {
                    http_202(
                        r#"{"requestId":"req-ack-123","result":"Request submitted successfully"}"#,
                    )
                } else if req.contains("/close") {
                    http_202(
                        r#"{"requestId":"req-close-123","result":"Request submitted successfully"}"#,
                    )
                } else {
                    http_202(
                        r#"{"requestId":"req-create-123","result":"Request submitted successfully"}"#,
                    )
                }
            } else {
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
            };

            let _ = stream.write_all(response.as_bytes());
        }
    });

    format!("http://127.0.0.1:{}", port)
}

/// Starts a focused Phase 2 mock server (Jira write operations + Agile reads).
pub fn start_mock_jira_write_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };

            let req = read_request(&mut stream);

            let response = if req.contains("POST /rest/api/3/issue\r\n")
                || req.contains("POST /rest/api/3/issue ")
            {
                // Create issue
                http_201(r#"{"id":"102","key":"TEST-102","self":"http://localhost/issue/102"}"#)
            } else if req.contains("DELETE /rest/api/3/issue/TEST-101/comment/") {
                http_204()
            } else if req.contains("DELETE /rest/api/3/issue/TEST-101/worklog/") {
                http_204()
            } else if req.contains("DELETE /rest/api/3/issue/TEST-101") {
                http_204()
            } else if req.contains("PUT /rest/api/3/issue/TEST-101/assignee") {
                http_204()
            } else if req.contains("POST /rest/api/3/issue/TEST-101/transitions") {
                http_204()
            } else if req.contains("GET /rest/api/3/issue/TEST-101/transitions") {
                http_ok(
                    r#"{"transitions":[{"id":"21","name":"In Progress"},{"id":"31","name":"Done"}]}"#,
                )
            } else if req.contains("POST /rest/api/3/issue/TEST-101/comment") {
                http_201(
                    r#"{"id":"comment-1","body":null,"author":{"displayName":"Tester"},"created":"2026-05-30T00:00:00Z","updated":"2026-05-30T00:00:00Z"}"#,
                )
            } else if req.contains("GET /rest/api/3/issue/TEST-101/comment") {
                http_ok(
                    r#"{
                    "startAt":0,"maxResults":50,"total":1,
                    "comments":[{"id":"comment-1","body":null,"author":{"displayName":"Tester"},"created":"2026-05-30T00:00:00Z","updated":"2026-05-30T00:00:00Z"}]
                }"#,
                )
            } else if req.contains("POST /rest/api/3/issue/TEST-101/worklog") {
                http_201(
                    r#"{"id":"worklog-1","timeSpent":"2h","timeSpentSeconds":7200,"comment":null,"author":{"displayName":"Tester"},"created":"2026-05-30T00:00:00Z","updated":"2026-05-30T00:00:00Z"}"#,
                )
            } else if req.contains("GET /rest/api/3/issue/TEST-101/worklog") {
                http_ok(
                    r#"{
                    "startAt":0,"maxResults":50,"total":1,
                    "worklogs":[{"id":"worklog-1","timeSpent":"2h","timeSpentSeconds":7200,"comment":null,"author":{"displayName":"Tester"},"created":"2026-05-30T00:00:00Z","updated":"2026-05-30T00:00:00Z"}]
                }"#,
                )
            } else if req.contains("PUT /rest/api/3/issue/TEST-101") {
                // Edit issue (notifyUsers=false query param)
                http_204()
            } else if req.contains("GET /rest/agile/1.0/board/42/sprint") {
                http_ok(
                    r#"{
                    "startAt":0,"maxResults":50,"total":2,"isLast":true,
                    "values":[
                        {"id":101,"name":"Sprint 1","state":"active","startDate":"2026-05-01T00:00:00Z","endDate":"2026-05-15T00:00:00Z"},
                        {"id":102,"name":"Sprint 2","state":"future","startDate":"2026-05-15T00:00:00Z","endDate":"2026-05-29T00:00:00Z"}
                    ]
                }"#,
                )
            } else if req.contains("GET /rest/agile/1.0/board") {
                http_ok(
                    r#"{
                    "startAt":0,"maxResults":50,"total":1,"isLast":true,
                    "values":[{"id":42,"name":"Rust Board","type":"scrum","location":{"projectKey":"TEST","projectName":"Test Project"}}]
                }"#,
                )
            } else if req.contains("GET /rest/agile/1.0/sprint/101/issue") {
                http_ok(
                    r#"{
                    "startAt":0,"maxResults":50,"total":1,
                    "issues":[{"id":"201","key":"TEST-201","fields":{
                        "summary":"Sprint task","description":null,
                        "issuetype":{"name":"Task"},"status":{"name":"To Do"},
                        "priority":{"name":"Medium"},"assignee":null,"reporter":null,
                        "created":null,"updated":null,"labels":[],"components":[],"fixVersions":[]
                    }}]
                }"#,
                )
            } else if req.contains("GET /rest/agile/1.0/epic/EPIC-1/issue") {
                http_ok(
                    r#"{
                    "startAt":0,"maxResults":50,"total":1,
                    "issues":[{"id":"301","key":"TEST-301","fields":{
                        "summary":"Epic child issue","description":null,
                        "issuetype":{"name":"Story"},"status":{"name":"In Progress"},
                        "priority":{"name":"High"},"assignee":null,"reporter":null,
                        "created":null,"updated":null,"labels":[],"components":[],"fixVersions":[]
                    }}]
                }"#,
                )
            } else {
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
            };

            let _ = stream.write_all(response.as_bytes());
        }
    });

    format!("http://127.0.0.1:{}", port)
}
