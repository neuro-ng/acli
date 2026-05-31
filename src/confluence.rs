use serde::{Deserialize, Serialize};

use crate::client::Client;

// ---------------------------------------------------------------------------
// Space types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Space {
    pub id: String,
    pub key: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub space_type: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "homepageId")]
    pub homepage_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpacesResponse {
    pub results: Vec<Space>,
    #[serde(rename = "_links")]
    pub links: Option<PaginationLinks>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaginationLinks {
    pub next: Option<String>,
}

// ---------------------------------------------------------------------------
// Page types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageBody {
    pub representation: Option<String>,
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageVersion {
    pub number: i32,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Page {
    pub id: String,
    pub title: String,
    #[serde(rename = "spaceId")]
    pub space_id: Option<String>,
    pub status: Option<String>,
    pub version: Option<PageVersion>,
    pub body: Option<PageBody>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "authorId")]
    pub author_id: Option<String>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PagesResponse {
    pub results: Vec<Page>,
    #[serde(rename = "_links")]
    pub links: Option<PaginationLinks>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatePageResponse {
    pub id: String,
    pub title: String,
    #[serde(rename = "spaceId")]
    pub space_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Space API
// ---------------------------------------------------------------------------

pub fn list_spaces(
    client: &Client,
    limit: i32,
    space_type: Option<&str>,
) -> Result<SpacesResponse, String> {
    let limit_str = limit.to_string();
    let mut q: Vec<(&str, &str)> = vec![("limit", &limit_str)];
    if let Some(t) = space_type {
        q.push(("type", t));
    }
    let resp = client.request("GET", "/wiki/api/v2/spaces", Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse spaces: {}. Response: {}", e, resp))
}

pub fn get_space(client: &Client, id: &str) -> Result<Space, String> {
    let path = format!("/wiki/api/v2/spaces/{}", id);
    let resp = client.request("GET", &path, None, None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse space: {}. Response: {}", e, resp))
}

pub fn create_space(
    client: &Client,
    name: &str,
    key: Option<&str>,
    description: Option<&str>,
) -> Result<Space, String> {
    let mut body = serde_json::json!({ "name": name });
    if let Some(k) = key {
        body["key"] = serde_json::Value::String(k.to_string());
    }
    if let Some(desc) = description {
        body["description"] = serde_json::json!({
            "representation": "plain",
            "value": desc
        });
    }
    let resp = client.request("POST", "/wiki/api/v2/spaces", None, Some(body))?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse created space: {}. Response: {}", e, resp))
}

pub fn list_space_pages(
    client: &Client,
    space_id: &str,
    title: Option<&str>,
    limit: i32,
) -> Result<PagesResponse, String> {
    let limit_str = limit.to_string();
    let mut q: Vec<(&str, &str)> = vec![("limit", &limit_str)];
    if let Some(t) = title {
        q.push(("title", t));
    }
    let path = format!("/wiki/api/v2/spaces/{}/pages", space_id);
    let resp = client.request("GET", &path, Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse space pages: {}. Response: {}", e, resp))
}

// ---------------------------------------------------------------------------
// Page API
// ---------------------------------------------------------------------------

pub fn list_pages(
    client: &Client,
    space_id: Option<&str>,
    title: Option<&str>,
    limit: i32,
) -> Result<PagesResponse, String> {
    let limit_str = limit.to_string();
    let mut q: Vec<(&str, &str)> = vec![("limit", &limit_str)];
    if let Some(sid) = space_id {
        q.push(("space-id", sid));
    }
    if let Some(t) = title {
        q.push(("title", t));
    }
    let resp = client.request("GET", "/wiki/api/v2/pages", Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse pages: {}. Response: {}", e, resp))
}

pub fn get_page(client: &Client, id: &str, with_body: bool) -> Result<Page, String> {
    let path = format!("/wiki/api/v2/pages/{}", id);
    let q: &[(&str, &str)] = if with_body {
        &[("body-format", "storage")]
    } else {
        &[]
    };
    let resp = client.request("GET", &path, Some(q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse page: {}. Response: {}", e, resp))
}

pub fn create_page(
    client: &Client,
    space_id: &str,
    title: &str,
    body_html: Option<&str>,
    parent_id: Option<&str>,
) -> Result<CreatePageResponse, String> {
    let mut body = serde_json::json!({
        "spaceId": space_id,
        "status": "current",
        "title": title
    });
    if let Some(pid) = parent_id {
        body["parentId"] = serde_json::Value::String(pid.to_string());
    }
    if let Some(html) = body_html {
        body["body"] = serde_json::json!({
            "representation": "storage",
            "value": html
        });
    }
    let resp = client.request("POST", "/wiki/api/v2/pages", None, Some(body))?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse created page: {}. Response: {}", e, resp))
}

pub fn update_page(
    client: &Client,
    id: &str,
    title: &str,
    version_number: i32,
    body_html: Option<&str>,
) -> Result<Page, String> {
    let path = format!("/wiki/api/v2/pages/{}", id);
    let mut body = serde_json::json!({
        "id": id,
        "status": "current",
        "title": title,
        "version": { "number": version_number }
    });
    if let Some(html) = body_html {
        body["body"] = serde_json::json!({
            "representation": "storage",
            "value": html
        });
    }
    let resp = client.request("PUT", &path, None, Some(body))?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse updated page: {}. Response: {}", e, resp))
}

pub fn delete_page(client: &Client, id: &str) -> Result<(), String> {
    let path = format!("/wiki/api/v2/pages/{}", id);
    client.request("DELETE", &path, None, None)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// XHTML storage format → plain text renderer
//
// Confluence stores content as XHTML. This renderer extracts readable text
// without pulling in an HTML parsing dependency.
// ---------------------------------------------------------------------------

pub fn render_storage(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    let mut list_counters: Vec<Option<usize>> = Vec::new(); // None = bullet, Some(n) = ordered

    while let Some(ch) = chars.next() {
        if ch != '<' {
            if ch == '&' {
                out.push_str(&decode_entity(&mut chars));
            } else {
                out.push(ch);
            }
            continue;
        }

        // Inside a tag — collect its name
        let mut tag = String::new();
        let mut is_closing = false;
        let mut is_self_closing = false;

        if chars.peek() == Some(&'/') {
            is_closing = true;
            chars.next();
        }

        // Read tag name (stop at whitespace, /, or >)
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '>' || c == '/' {
                break;
            }
            tag.push(chars.next().unwrap());
        }

        // Skip to end of tag, detecting self-closing
        for c in chars.by_ref() {
            if c == '>' {
                break;
            }
            if c == '/' {
                is_self_closing = true;
            }
        }

        let tag_lower = tag.to_lowercase();

        if is_closing {
            match tag_lower.as_str() {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => out.push('\n'),
                "p" | "div" | "section" | "article" => {
                    trim_trailing_spaces(&mut out);
                    out.push('\n');
                }
                "li" => {
                    trim_trailing_spaces(&mut out);
                    out.push('\n');
                }
                "ul" | "ol" => {
                    list_counters.pop();
                    out.push('\n');
                }
                "td" | "th" => out.push('\t'),
                "tr" => {
                    trim_trailing_spaces(&mut out);
                    out.push('\n');
                }
                "table" => out.push('\n'),
                "pre" | "code" => out.push('\n'),
                _ => {}
            }
        } else if !is_self_closing {
            match tag_lower.as_str() {
                "h1" => out.push_str("# "),
                "h2" => out.push_str("## "),
                "h3" => out.push_str("### "),
                "h4" | "h5" | "h6" => out.push_str("#### "),
                "p" | "div" | "section" | "article" => {
                    ensure_blank_line(&mut out);
                }
                "br" => out.push('\n'),
                "li" => {
                    let indent = "  ".repeat(list_counters.len().saturating_sub(1));
                    let marker = match list_counters.last_mut() {
                        Some(Some(ref mut n)) => {
                            *n += 1;
                            format!("{}{}. ", indent, n)
                        }
                        Some(None) => format!("{}* ", indent),
                        None => "* ".to_string(),
                    };
                    trim_trailing_spaces(&mut out);
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str(&marker);
                }
                "ul" => list_counters.push(None),
                "ol" => list_counters.push(Some(0)),
                "hr" | "ac:task-list" => {
                    out.push_str("---\n");
                }
                // Confluence macro wrapper — skip (content inside is rendered normally)
                "ac:structured-macro"
                | "ac:parameter"
                | "ac:plain-text-body"
                | "ac:rich-text-body"
                | "ri:attachment"
                | "ri:page" => {}
                _ => {}
            }
        }
    }

    // Normalise: collapse 3+ consecutive newlines to 2
    let mut result = String::with_capacity(out.len());
    let mut newline_run = 0usize;
    for ch in out.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                result.push('\n');
            }
        } else {
            newline_run = 0;
            result.push(ch);
        }
    }

    result.trim().to_string()
}

fn trim_trailing_spaces(s: &mut String) {
    let trimmed_len = s.trim_end_matches(' ').len();
    s.truncate(trimmed_len);
}

fn ensure_blank_line(s: &mut String) {
    if !s.is_empty() && !s.ends_with("\n\n") {
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push('\n');
    }
}

/// Decodes an HTML entity (cursor is just past the `&`).
fn decode_entity(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut entity = String::new();
    for c in chars.by_ref() {
        if c == ';' {
            break;
        }
        entity.push(c);
    }
    match entity.as_str() {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" | "#39" => "'".to_string(),
        "nbsp" => " ".to_string(),
        "ndash" => "\u{2013}".to_string(),
        "mdash" => "\u{2014}".to_string(),
        "hellip" => "\u{2026}".to_string(),
        s if s.starts_with('#') => {
            let code_str = &s[1..];
            let code: u32 = if let Some(hex) = code_str.strip_prefix('x') {
                u32::from_str_radix(hex, 16).unwrap_or(0)
            } else {
                code_str.parse().unwrap_or(0)
            };
            char::from_u32(code)
                .map(|c| c.to_string())
                .unwrap_or_default()
        }
        _ => format!("&{};", entity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plain_paragraph() {
        assert_eq!(render_storage("<p>Hello world</p>"), "Hello world");
    }

    #[test]
    fn test_render_heading() {
        assert_eq!(render_storage("<h1>Title</h1>"), "# Title");
        assert_eq!(render_storage("<h2>Subtitle</h2>"), "## Subtitle");
    }

    #[test]
    fn test_render_bullet_list() {
        let html = "<ul><li>Alpha</li><li>Beta</li></ul>";
        let rendered = render_storage(html);
        assert!(rendered.contains("* Alpha"));
        assert!(rendered.contains("* Beta"));
    }

    #[test]
    fn test_render_ordered_list() {
        let html = "<ol><li>First</li><li>Second</li></ol>";
        let rendered = render_storage(html);
        assert!(rendered.contains("1. First"));
        assert!(rendered.contains("2. Second"));
    }

    #[test]
    fn test_render_entity_decoding() {
        assert_eq!(render_storage("a &amp; b"), "a & b");
        assert_eq!(render_storage("&lt;tag&gt;"), "<tag>");
        assert_eq!(render_storage("&quot;quoted&quot;"), "\"quoted\"");
    }

    #[test]
    fn test_render_strips_unknown_tags() {
        assert_eq!(render_storage("<strong>bold</strong>"), "bold");
        assert_eq!(render_storage("<em>italic</em>"), "italic");
    }

    #[test]
    fn test_render_empty() {
        assert_eq!(render_storage(""), "");
        assert_eq!(render_storage("<p></p>"), "");
    }

    #[test]
    fn test_render_numeric_entity() {
        assert_eq!(render_storage("&#65;"), "A");
        assert_eq!(render_storage("&#x41;"), "A");
    }

    #[test]
    fn test_render_collapses_excess_newlines() {
        let html = "<p>First</p><p>Second</p>";
        let rendered = render_storage(html);
        // Should not have more than two consecutive newlines
        assert!(!rendered.contains("\n\n\n"));
    }
}
