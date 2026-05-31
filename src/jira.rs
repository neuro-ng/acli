use crate::client::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IssueType {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusDetails {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Priority {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDetails {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectComponent {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Version {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IssueFields {
    pub summary: String,
    pub description: Option<serde_json::Value>,
    pub issuetype: Option<IssueType>,
    pub status: Option<StatusDetails>,
    pub priority: Option<Priority>,
    pub assignee: Option<UserDetails>,
    pub reporter: Option<UserDetails>,
    pub created: Option<String>,
    pub updated: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub components: Vec<ProjectComponent>,
    #[serde(rename = "fixVersions", default)]
    pub fix_versions: Vec<Version>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IssueDetailed {
    pub id: String,
    pub key: String,
    pub fields: IssueFields,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchResults {
    #[serde(rename = "startAt")]
    pub start_at: i32,
    #[serde(rename = "maxResults")]
    pub max_results: i32,
    pub total: i32,
    pub issues: Vec<IssueDetailed>,
}

pub fn get_issue(client: &Client, key: &str) -> Result<IssueDetailed, String> {
    let path = format!("/rest/api/3/issue/{}", key);
    let resp = client.request("GET", &path, None, None)?;
    let issue: IssueDetailed = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse issue JSON: {}. Response: {}", e, resp))?;
    Ok(issue)
}

pub fn search_jql(
    client: &Client,
    jql: &str,
    start_at: i32,
    max_results: i32,
    fields: &[&str],
) -> Result<SearchResults, String> {
    let fields_str = fields.join(",");
    let start_at_str = start_at.to_string();
    let max_results_str = max_results.to_string();
    let query = [
        ("jql", jql),
        ("startAt", &start_at_str),
        ("maxResults", &max_results_str),
        ("fields", &fields_str),
    ];
    let resp = client.request("GET", "/rest/api/3/search/jql", Some(&query), None)?;
    let results: SearchResults = serde_json::from_str(&resp).map_err(|e| {
        format!(
            "Failed to parse search results JSON: {}. Response: {}",
            e, resp
        )
    })?;
    Ok(results)
}

// ADF (Atlassian Document Format) Plain Text Renderer
pub fn render_adf(doc: &serde_json::Value) -> String {
    match doc {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => {
            let mut result = render_node(map);
            if result.ends_with('\n') {
                result.pop();
            }
            result
        }
        _ => doc.to_string(),
    }
}

// --- Milestone 2: Issue Write Operations ---

/// Converts plain text to a minimal ADF document (reverse of render_adf).
pub fn text_to_adf(text: &str) -> serde_json::Value {
    let paragraphs: Vec<serde_json::Value> = text
        .split('\n')
        .map(|line| {
            serde_json::json!({
                "type": "paragraph",
                "content": [{ "type": "text", "text": line }]
            })
        })
        .collect();

    serde_json::json!({
        "type": "doc",
        "version": 1,
        "content": paragraphs
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateIssueResponse {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub self_url: Option<String>,
}

pub fn create_issue(
    client: &Client,
    project: &str,
    summary: &str,
    issue_type: &str,
    description: Option<&str>,
    priority: Option<&str>,
) -> Result<CreateIssueResponse, String> {
    let mut fields = serde_json::json!({
        "project": { "key": project },
        "issuetype": { "name": issue_type },
        "summary": summary
    });

    if let Some(desc) = description {
        fields["description"] = text_to_adf(desc);
    }
    if let Some(pri) = priority {
        fields["priority"] = serde_json::json!({ "name": pri });
    }

    let body = serde_json::json!({ "fields": fields });
    let resp = client.request("POST", "/rest/api/3/issue", None, Some(body))?;
    let result: CreateIssueResponse = serde_json::from_str(&resp).map_err(|e| {
        format!(
            "Failed to parse create issue response: {}. Response: {}",
            e, resp
        )
    })?;
    Ok(result)
}

pub fn edit_issue(
    client: &Client,
    key: &str,
    summary: Option<&str>,
    description: Option<&str>,
    priority: Option<&str>,
) -> Result<(), String> {
    let mut fields = serde_json::Map::new();
    if let Some(s) = summary {
        fields.insert(
            "summary".to_string(),
            serde_json::Value::String(s.to_string()),
        );
    }
    if let Some(desc) = description {
        fields.insert("description".to_string(), text_to_adf(desc));
    }
    if let Some(pri) = priority {
        fields.insert("priority".to_string(), serde_json::json!({ "name": pri }));
    }

    let body = serde_json::json!({ "fields": fields });
    let path = format!("/rest/api/3/issue/{}?notifyUsers=false", key);
    client.request("PUT", &path, None, Some(body))?;
    Ok(())
}

pub fn delete_issue(client: &Client, key: &str, delete_subtasks: bool) -> Result<(), String> {
    let path = format!(
        "/rest/api/3/issue/{}?deleteSubtasks={}",
        key, delete_subtasks
    );
    client.request("DELETE", &path, None, None)?;
    Ok(())
}

pub fn assign_issue(client: &Client, key: &str, account_id: Option<&str>) -> Result<(), String> {
    let body = match account_id {
        Some(id) => serde_json::json!({ "accountId": id }),
        None => serde_json::json!({ "accountId": null }),
    };
    let path = format!("/rest/api/3/issue/{}/assignee", key);
    client.request("PUT", &path, None, Some(body))?;
    Ok(())
}

// --- Transitions ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transition {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransitionsResponse {
    pub transitions: Vec<Transition>,
}

pub fn get_transitions(client: &Client, key: &str) -> Result<Vec<Transition>, String> {
    let path = format!("/rest/api/3/issue/{}/transitions", key);
    let resp = client.request("GET", &path, None, None)?;
    let result: TransitionsResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse transitions: {}. Response: {}", e, resp))?;
    Ok(result.transitions)
}

pub fn do_transition(client: &Client, key: &str, transition_id: &str) -> Result<(), String> {
    let path = format!("/rest/api/3/issue/{}/transitions", key);
    let body = serde_json::json!({ "transition": { "id": transition_id } });
    client.request("POST", &path, None, Some(body))?;
    Ok(())
}

// --- Comments ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommentAuthor {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Comment {
    pub id: String,
    pub body: Option<serde_json::Value>,
    pub author: Option<CommentAuthor>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommentsResponse {
    #[serde(rename = "startAt")]
    pub start_at: i32,
    #[serde(rename = "maxResults")]
    pub max_results: i32,
    pub total: i32,
    pub comments: Vec<Comment>,
}

pub fn list_comments(client: &Client, key: &str) -> Result<Vec<Comment>, String> {
    let path = format!("/rest/api/3/issue/{}/comment", key);
    let resp = client.request("GET", &path, None, None)?;
    let result: CommentsResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse comments: {}. Response: {}", e, resp))?;
    Ok(result.comments)
}

pub fn add_comment(client: &Client, key: &str, body_text: &str) -> Result<Comment, String> {
    let path = format!("/rest/api/3/issue/{}/comment", key);
    let body = serde_json::json!({ "body": text_to_adf(body_text) });
    let resp = client.request("POST", &path, None, Some(body))?;
    let comment: Comment = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse created comment: {}. Response: {}", e, resp))?;
    Ok(comment)
}

pub fn delete_comment(client: &Client, key: &str, comment_id: &str) -> Result<(), String> {
    let path = format!("/rest/api/3/issue/{}/comment/{}", key, comment_id);
    client.request("DELETE", &path, None, None)?;
    Ok(())
}

// --- Worklogs ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorklogAuthor {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Worklog {
    pub id: String,
    #[serde(rename = "timeSpent")]
    pub time_spent: Option<String>,
    #[serde(rename = "timeSpentSeconds")]
    pub time_spent_seconds: Option<i64>,
    pub comment: Option<serde_json::Value>,
    pub author: Option<WorklogAuthor>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorklogsResponse {
    #[serde(rename = "startAt")]
    pub start_at: i32,
    #[serde(rename = "maxResults")]
    pub max_results: i32,
    pub total: i32,
    pub worklogs: Vec<Worklog>,
}

pub fn list_worklogs(client: &Client, key: &str) -> Result<Vec<Worklog>, String> {
    let path = format!("/rest/api/3/issue/{}/worklog", key);
    let resp = client.request("GET", &path, None, None)?;
    let result: WorklogsResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse worklogs: {}. Response: {}", e, resp))?;
    Ok(result.worklogs)
}

pub fn add_worklog(
    client: &Client,
    key: &str,
    time_spent: &str,
    comment: Option<&str>,
) -> Result<Worklog, String> {
    let path = format!("/rest/api/3/issue/{}/worklog", key);
    let mut body = serde_json::json!({ "timeSpent": time_spent });
    if let Some(c) = comment {
        body["comment"] = text_to_adf(c);
    }
    let resp = client.request("POST", &path, None, Some(body))?;
    let worklog: Worklog = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse created worklog: {}. Response: {}", e, resp))?;
    Ok(worklog)
}

pub fn delete_worklog(client: &Client, key: &str, worklog_id: &str) -> Result<(), String> {
    let path = format!("/rest/api/3/issue/{}/worklog/{}", key, worklog_id);
    client.request("DELETE", &path, None, None)?;
    Ok(())
}

// --- Attachments ---

pub fn attach_file(client: &Client, key: &str, file_path: &str) -> Result<String, String> {
    let path = format!("/rest/api/3/issue/{}/attachments", key);
    client.request_multipart("POST", &path, file_path)
}

fn render_node(node: &serde_json::Map<String, serde_json::Value>) -> String {
    let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match node_type {
        "doc" | "blockquote" | "panel" | "expand" => render_children(node),
        "paragraph" | "heading" | "codeBlock" => {
            let mut s = render_children(node);
            s.push('\n');
            s
        }
        "bulletList" => render_bullet_list(node),
        "orderedList" => render_ordered_list(node),
        "listItem" => render_list_item(node),
        "text" => node
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string(),
        "hardBreak" => "\n".to_string(),
        "rule" => "---\n".to_string(),
        "table" => render_children(node),
        "tableRow" => render_table_row(node),
        "tableHeader" | "tableCell" => {
            let mut s = render_children(node);
            if s.ends_with('\n') {
                s.pop();
            }
            s
        }
        "mention" => node_attr(node, "text", "@unknown"),
        "inlineCard" => node_attr(node, "url", ""),
        "emoji" => {
            let text = node_attr(node, "text", "");
            if !text.is_empty() {
                text
            } else {
                node_attr(node, "shortName", "")
            }
        }
        "date" => node_attr(node, "timestamp", ""),
        "status" => node_attr(node, "text", ""),
        "mediaSingle" | "mediaInline" | "media" | "mediaGroup" => String::new(),
        _ => render_children(node),
    }
}

fn render_children(node: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut sb = String::new();
    if let Some(serde_json::Value::Array(content)) = node.get("content") {
        for child in content {
            if let serde_json::Value::Object(map) = child {
                sb.push_str(&render_node(map));
            }
        }
    }
    sb
}

fn render_bullet_list(node: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut sb = String::new();
    if let Some(serde_json::Value::Array(content)) = node.get("content") {
        for child in content {
            if let serde_json::Value::Object(map) = child {
                let text = render_node(map);
                let trimmed = text.trim_end_matches('\n');
                sb.push_str(&format!("* {}\n", trimmed));
            }
        }
    }
    sb
}

fn render_ordered_list(node: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut sb = String::new();
    if let Some(serde_json::Value::Array(content)) = node.get("content") {
        for (i, child) in content.iter().enumerate() {
            if let serde_json::Value::Object(map) = child {
                let text = render_node(map);
                let trimmed = text.trim_end_matches('\n');
                sb.push_str(&format!("{}. {}\n", i + 1, trimmed));
            }
        }
    }
    sb
}

fn render_list_item(node: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut sb = String::new();
    if let Some(serde_json::Value::Array(content)) = node.get("content") {
        let mut first = true;
        for child in content {
            if let serde_json::Value::Object(map) = child {
                let text = render_node(map);
                let trimmed = text.trim_end_matches('\n');
                if !trimmed.is_empty() {
                    if !first {
                        sb.push('\n');
                    }
                    sb.push_str(trimmed);
                    first = false;
                }
            }
        }
    }
    sb
}

fn render_table_row(node: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut cells = Vec::new();
    if let Some(serde_json::Value::Array(content)) = node.get("content") {
        for child in content {
            if let serde_json::Value::Object(map) = child {
                cells.push(render_node(map));
            }
        }
    }
    format!("{}\n", cells.join("\t"))
}

fn node_attr(
    node: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: &str,
) -> String {
    node.get("attrs")
        .and_then(|a| a.as_object())
        .and_then(|o| o.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_to_adf_single_line() {
        let adf = text_to_adf("Hello world");
        assert_eq!(adf["type"], "doc");
        assert_eq!(adf["version"], 1);
        assert_eq!(adf["content"][0]["type"], "paragraph");
        assert_eq!(adf["content"][0]["content"][0]["text"], "Hello world");
    }

    #[test]
    fn test_text_to_adf_multi_line() {
        let adf = text_to_adf("Line 1\nLine 2");
        assert_eq!(adf["content"][0]["content"][0]["text"], "Line 1");
        assert_eq!(adf["content"][1]["content"][0]["text"], "Line 2");
    }

    #[test]
    fn test_render_adf_plain_paragraph() {
        let doc = serde_json::json!({
            "type": "doc",
            "version": 1,
            "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Hello world"}]}]
        });
        assert_eq!(render_adf(&doc), "Hello world");
    }

    #[test]
    fn test_render_adf_two_paragraphs() {
        let doc = serde_json::json!({
            "type": "doc",
            "version": 1,
            "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": "Line 1"}]},
                {"type": "paragraph", "content": [{"type": "text", "text": "Line 2"}]}
            ]
        });
        assert_eq!(render_adf(&doc), "Line 1\nLine 2");
    }

    #[test]
    fn test_render_adf_hard_break() {
        let doc = serde_json::json!({
            "type": "doc",
            "version": 1,
            "content": [{"type": "paragraph", "content": [
                {"type": "text", "text": "Hello"},
                {"type": "hardBreak"},
                {"type": "text", "text": "World"}
            ]}]
        });
        assert_eq!(render_adf(&doc), "Hello\nWorld");
    }

    #[test]
    fn test_render_adf_rule() {
        let doc = serde_json::json!({
            "type": "doc",
            "version": 1,
            "content": [{"type": "rule"}]
        });
        assert_eq!(render_adf(&doc), "---");
    }

    #[test]
    fn test_render_adf_bullet_list() {
        let doc = serde_json::json!({
            "type": "doc",
            "version": 1,
            "content": [{"type": "bulletList", "content": [
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Item 1"}]}]},
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Item 2"}]}]}
            ]}]
        });
        assert_eq!(render_adf(&doc), "* Item 1\n* Item 2");
    }

    #[test]
    fn test_render_adf_ordered_list() {
        let doc = serde_json::json!({
            "type": "doc",
            "version": 1,
            "content": [{"type": "orderedList", "content": [
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "First"}]}]},
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Second"}]}]}
            ]}]
        });
        assert_eq!(render_adf(&doc), "1. First\n2. Second");
    }

    #[test]
    fn test_render_adf_string_passthrough() {
        let s = serde_json::Value::String("plain text".to_string());
        assert_eq!(render_adf(&s), "plain text");
    }

    #[test]
    fn test_roundtrip_text_to_adf_and_render() {
        let original = "Line A\nLine B";
        let adf = text_to_adf(original);
        let rendered = render_adf(&adf);
        assert_eq!(rendered, original);
    }
}
