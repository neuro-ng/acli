mod common;

use std::io::Write;
use std::net::TcpListener;
use std::thread;

use acli_rust::client::Client;
use acli_rust::confluence;
use common::{http_201, http_204, http_ok, mock_profile, read_request};

pub fn start_mock_confluence_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };

            let req = read_request(&mut stream);

            let response = if req.contains("POST /wiki/api/v2/spaces\r\n")
                || req.contains("POST /wiki/api/v2/spaces ")
            {
                http_201(
                    r#"{"id":"SPACE-42","key":"TEST","name":"Test Space","type":"global","status":"current"}"#,
                )
            } else if req.contains("GET /wiki/api/v2/spaces/SPACE-1/pages") {
                http_ok(
                    r#"{
                    "results":[
                        {"id":"PAGE-101","title":"Child Page","spaceId":"SPACE-1","status":"current","version":{"number":1},"createdAt":"2026-05-30T00:00:00Z","authorId":"user-1"}
                    ],
                    "_links":{}
                }"#,
                )
            } else if req.contains("GET /wiki/api/v2/spaces/SPACE-1") {
                http_ok(
                    r#"{"id":"SPACE-1","key":"TEST","name":"Test Space","type":"global","status":"current","homepageId":"PAGE-100"}"#,
                )
            } else if req.contains("GET /wiki/api/v2/spaces") {
                http_ok(
                    r#"{
                    "results":[
                        {"id":"SPACE-1","key":"TEST","name":"Test Space","type":"global","status":"current"},
                        {"id":"SPACE-2","key":"DEMO","name":"Demo Space","type":"global","status":"current"}
                    ],
                    "_links":{}
                }"#,
                )
            } else if req.contains("POST /wiki/api/v2/pages\r\n")
                || req.contains("POST /wiki/api/v2/pages ")
            {
                http_201(r#"{"id":"PAGE-200","title":"New Page","spaceId":"SPACE-1"}"#)
            } else if req.contains("DELETE /wiki/api/v2/pages/") {
                http_204()
            } else if req.contains("PUT /wiki/api/v2/pages/PAGE-101") {
                http_ok(
                    r#"{"id":"PAGE-101","title":"Updated Title","spaceId":"SPACE-1","status":"current","version":{"number":2},"createdAt":"2026-05-30T00:00:00Z","authorId":"user-1"}"#,
                )
            } else if req.contains("GET /wiki/api/v2/pages/PAGE-101") {
                http_ok(
                    r#"{"id":"PAGE-101","title":"Child Page","spaceId":"SPACE-1","status":"current","version":{"number":1},"body":{"representation":"storage","value":"<p>Hello <strong>World</strong></p>"},"createdAt":"2026-05-30T00:00:00Z","authorId":"user-1"}"#,
                )
            } else if req.contains("GET /wiki/api/v2/pages") {
                http_ok(
                    r#"{
                    "results":[
                        {"id":"PAGE-101","title":"Child Page","spaceId":"SPACE-1","status":"current","version":{"number":1},"createdAt":"2026-05-30T00:00:00Z","authorId":"user-1"}
                    ],
                    "_links":{}
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

#[test]
fn test_confluence_list_spaces() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    let spaces = confluence::list_spaces(&client, 50, None).unwrap();
    assert_eq!(spaces.results.len(), 2);
    assert_eq!(spaces.results[0].id, "SPACE-1");
    assert_eq!(spaces.results[0].name, "Test Space");
    assert_eq!(spaces.results[0].key.as_deref(), Some("TEST"));
}

#[test]
fn test_confluence_get_space() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    let space = confluence::get_space(&client, "SPACE-1").unwrap();
    assert_eq!(space.id, "SPACE-1");
    assert_eq!(space.name, "Test Space");
    assert_eq!(space.space_type.as_deref(), Some("global"));
}

#[test]
fn test_confluence_create_space() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    let space = confluence::create_space(&client, "Test Space", Some("TEST"), Some("A test space"))
        .unwrap();
    assert_eq!(space.id, "SPACE-42");
    assert_eq!(space.name, "Test Space");
}

#[test]
fn test_confluence_list_space_pages() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    let pages = confluence::list_space_pages(&client, "SPACE-1", None, 50).unwrap();
    assert_eq!(pages.results.len(), 1);
    assert_eq!(pages.results[0].id, "PAGE-101");
    assert_eq!(pages.results[0].title, "Child Page");
}

#[test]
fn test_confluence_list_pages() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    let pages = confluence::list_pages(&client, Some("SPACE-1"), None, 50).unwrap();
    assert_eq!(pages.results.len(), 1);
    assert_eq!(pages.results[0].id, "PAGE-101");
    assert_eq!(pages.results[0].title, "Child Page");
}

#[test]
fn test_confluence_get_page() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    let page = confluence::get_page(&client, "PAGE-101", true).unwrap();
    assert_eq!(page.id, "PAGE-101");
    assert_eq!(page.title, "Child Page");
    assert_eq!(page.version.as_ref().unwrap().number, 1);

    let body = page.body.as_ref().unwrap();
    assert_eq!(body.representation.as_deref(), Some("storage"));
    // Body content should be present
    assert!(body.value.as_ref().unwrap().contains("Hello"));
}

#[test]
fn test_confluence_create_page() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    let created = confluence::create_page(
        &client,
        "SPACE-1",
        "New Page",
        Some("<p>Hello Confluence</p>"),
        None,
    )
    .unwrap();
    assert_eq!(created.id, "PAGE-200");
    assert_eq!(created.title, "New Page");
}

#[test]
fn test_confluence_update_page() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    let page = confluence::update_page(
        &client,
        "PAGE-101",
        "Updated Title",
        2,
        Some("<p>Updated content</p>"),
    )
    .unwrap();
    assert_eq!(page.title, "Updated Title");
    assert_eq!(page.version.as_ref().unwrap().number, 2);
}

#[test]
fn test_confluence_delete_page() {
    let url = start_mock_confluence_server();
    let client = Client::new(mock_profile(&url));

    confluence::delete_page(&client, "PAGE-101").unwrap();
}

#[test]
fn test_render_storage_with_real_html() {
    use acli_rust::confluence::render_storage;

    let html = r#"<h1>Title</h1><p>First paragraph.</p><ul><li>Item A</li><li>Item B</li></ul><p>Second paragraph with &amp; entity.</p>"#;
    let rendered = render_storage(html);

    assert!(rendered.contains("# Title"));
    assert!(rendered.contains("First paragraph."));
    assert!(rendered.contains("* Item A"));
    assert!(rendered.contains("* Item B"));
    assert!(rendered.contains("Second paragraph with & entity."));
}
