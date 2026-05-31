use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::jira::SearchResults;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoardLocation {
    #[serde(rename = "projectKey", default)]
    pub project_key: String,
    #[serde(rename = "projectName", default)]
    pub project_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Board {
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub board_type: String,
    pub location: Option<BoardLocation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoardsResponse {
    #[serde(rename = "startAt")]
    pub start_at: i32,
    #[serde(rename = "maxResults")]
    pub max_results: i32,
    pub total: i32,
    #[serde(rename = "isLast", default)]
    pub is_last: bool,
    pub values: Vec<Board>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sprint {
    pub id: i32,
    pub name: String,
    pub state: String,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SprintsResponse {
    #[serde(rename = "startAt")]
    pub start_at: i32,
    #[serde(rename = "maxResults")]
    pub max_results: i32,
    pub total: i32,
    #[serde(rename = "isLast", default)]
    pub is_last: bool,
    pub values: Vec<Sprint>,
}

pub fn list_boards(
    client: &Client,
    start_at: i32,
    max_results: i32,
    project: Option<&str>,
) -> Result<BoardsResponse, String> {
    let start_str = start_at.to_string();
    let max_str = max_results.to_string();

    let mut q: Vec<(&str, &str)> = vec![("startAt", &start_str), ("maxResults", &max_str)];
    if let Some(proj) = project {
        q.push(("projectKeyOrId", proj));
    }

    let resp = client.request("GET", "/rest/agile/1.0/board", Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse boards: {}. Response: {}", e, resp))
}

pub fn get_board_sprints(
    client: &Client,
    board_id: i32,
    start_at: i32,
    max_results: i32,
) -> Result<SprintsResponse, String> {
    let start_str = start_at.to_string();
    let max_str = max_results.to_string();
    let path = format!("/rest/agile/1.0/board/{}/sprint", board_id);
    let q = [
        ("startAt", start_str.as_str()),
        ("maxResults", max_str.as_str()),
    ];

    let resp = client.request("GET", &path, Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse sprints: {}. Response: {}", e, resp))
}

pub fn get_sprint_issues(
    client: &Client,
    sprint_id: i32,
    start_at: i32,
    max_results: i32,
) -> Result<SearchResults, String> {
    let start_str = start_at.to_string();
    let max_str = max_results.to_string();
    let path = format!("/rest/agile/1.0/sprint/{}/issue", sprint_id);
    let q = [
        ("startAt", start_str.as_str()),
        ("maxResults", max_str.as_str()),
    ];

    let resp = client.request("GET", &path, Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse sprint issues: {}. Response: {}", e, resp))
}

pub fn get_epic_issues(
    client: &Client,
    epic_key: &str,
    start_at: i32,
    max_results: i32,
) -> Result<SearchResults, String> {
    let start_str = start_at.to_string();
    let max_str = max_results.to_string();
    let path = format!("/rest/agile/1.0/epic/{}/issue", epic_key);
    let q = [
        ("startAt", start_str.as_str()),
        ("maxResults", max_str.as_str()),
    ];

    let resp = client.request("GET", &path, Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse epic issues: {}. Response: {}", e, resp))
}
