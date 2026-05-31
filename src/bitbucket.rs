use serde::{Deserialize, Serialize};

use crate::client::Client;

const BB_BASE: &str = "https://api.bitbucket.org/2.0";

fn bb_url(path: &str) -> String {
    format!("{}{}", BB_BASE, path)
}

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BbUser {
    #[serde(rename = "display_name")]
    pub display_name: String,
    pub uuid: Option<String>,
    pub nickname: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BbRef {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BbCommit {
    pub hash: String,
}

// ---------------------------------------------------------------------------
// Repository types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MainBranch {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Repo {
    #[serde(rename = "full_name")]
    pub full_name: String,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub scm: Option<String>,
    #[serde(rename = "is_private")]
    pub is_private: bool,
    #[serde(rename = "created_on")]
    pub created_on: Option<String>,
    #[serde(rename = "updated_on")]
    pub updated_on: Option<String>,
    #[serde(rename = "mainbranch")]
    pub main_branch: Option<MainBranch>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReposPage {
    pub values: Vec<Repo>,
    pub next: Option<String>,
    #[serde(rename = "pagelen")]
    pub page_len: Option<i32>,
    pub size: Option<i32>,
}

// ---------------------------------------------------------------------------
// Pull request types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrEndpoint {
    pub branch: BbRef,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PullRequest {
    pub id: i64,
    pub title: String,
    pub state: String,
    pub author: BbUser,
    pub source: PrEndpoint,
    pub destination: PrEndpoint,
    pub description: Option<String>,
    #[serde(rename = "created_on")]
    pub created_on: Option<String>,
    #[serde(rename = "updated_on")]
    pub updated_on: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrsPage {
    pub values: Vec<PullRequest>,
    pub next: Option<String>,
    pub size: Option<i32>,
}

// ---------------------------------------------------------------------------
// Pipeline types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipelineStateResult {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipelineState {
    pub name: String,
    pub result: Option<PipelineStateResult>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipelineTrigger {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipelineTarget {
    #[serde(rename = "ref_name")]
    pub ref_name: Option<String>,
    pub commit: Option<BbCommit>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pipeline {
    pub uuid: String,
    #[serde(rename = "build_number")]
    pub build_number: i64,
    pub state: PipelineState,
    pub trigger: PipelineTrigger,
    pub target: PipelineTarget,
    #[serde(rename = "created_on")]
    pub created_on: Option<String>,
    #[serde(rename = "completed_on")]
    pub completed_on: Option<String>,
    #[serde(rename = "duration_in_seconds")]
    pub duration_in_seconds: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipelinesPage {
    pub values: Vec<Pipeline>,
    pub next: Option<String>,
    pub size: Option<i32>,
}

// ---------------------------------------------------------------------------
// Repo API
// ---------------------------------------------------------------------------

pub fn list_repos(
    client: &Client,
    workspace: &str,
    role: Option<&str>,
    query: Option<&str>,
    page_len: i32,
) -> Result<ReposPage, String> {
    let path = format!("/repositories/{}", workspace);
    let url = bb_url(&path);
    let len_str = page_len.to_string();
    let mut q: Vec<(&str, &str)> = vec![("pagelen", &len_str)];
    if let Some(r) = role {
        q.push(("role", r));
    }
    if let Some(f) = query {
        q.push(("q", f));
    }
    let resp = client.request("GET", &url, Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse repos: {}. Response: {}", e, resp))
}

pub fn get_repo(client: &Client, workspace: &str, slug: &str) -> Result<Repo, String> {
    let url = bb_url(&format!("/repositories/{}/{}", workspace, slug));
    let resp = client.request("GET", &url, None, None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse repo: {}. Response: {}", e, resp))
}

pub fn create_repo(
    client: &Client,
    workspace: &str,
    slug: &str,
    name: &str,
    is_private: bool,
    description: Option<&str>,
    language: Option<&str>,
) -> Result<Repo, String> {
    let url = bb_url(&format!("/repositories/{}/{}", workspace, slug));
    let mut body = serde_json::json!({
        "scm": "git",
        "name": name,
        "is_private": is_private
    });
    if let Some(d) = description {
        body["description"] = serde_json::Value::String(d.to_string());
    }
    if let Some(l) = language {
        body["language"] = serde_json::Value::String(l.to_string());
    }
    let resp = client.request("POST", &url, None, Some(body))?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse created repo: {}. Response: {}", e, resp))
}

pub fn delete_repo(client: &Client, workspace: &str, slug: &str) -> Result<(), String> {
    let url = bb_url(&format!("/repositories/{}/{}", workspace, slug));
    client.request("DELETE", &url, None, None)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// PR API
// ---------------------------------------------------------------------------

pub fn list_prs(
    client: &Client,
    workspace: &str,
    slug: &str,
    state: Option<&str>,
    page_len: i32,
) -> Result<PrsPage, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pullrequests",
        workspace, slug
    ));
    let len_str = page_len.to_string();
    let mut q: Vec<(&str, &str)> = vec![("pagelen", &len_str)];
    if let Some(s) = state {
        q.push(("state", s));
    }
    let resp = client.request("GET", &url, Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse PRs: {}. Response: {}", e, resp))
}

pub fn get_pr(
    client: &Client,
    workspace: &str,
    slug: &str,
    pr_id: i64,
) -> Result<PullRequest, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pullrequests/{}",
        workspace, slug, pr_id
    ));
    let resp = client.request("GET", &url, None, None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse PR: {}. Response: {}", e, resp))
}

pub fn create_pr(
    client: &Client,
    workspace: &str,
    slug: &str,
    title: &str,
    source_branch: &str,
    dest_branch: &str,
    description: Option<&str>,
) -> Result<PullRequest, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pullrequests",
        workspace, slug
    ));
    let mut body = serde_json::json!({
        "title": title,
        "source": { "branch": { "name": source_branch } },
        "destination": { "branch": { "name": dest_branch } }
    });
    if let Some(d) = description {
        body["description"] = serde_json::Value::String(d.to_string());
    }
    let resp = client.request("POST", &url, None, Some(body))?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse created PR: {}. Response: {}", e, resp))
}

pub fn approve_pr(
    client: &Client,
    workspace: &str,
    slug: &str,
    pr_id: i64,
) -> Result<String, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pullrequests/{}/approve",
        workspace, slug, pr_id
    ));
    client.request("POST", &url, None, None)
}

pub fn merge_pr(
    client: &Client,
    workspace: &str,
    slug: &str,
    pr_id: i64,
    merge_strategy: Option<&str>,
) -> Result<PullRequest, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pullrequests/{}/merge",
        workspace, slug, pr_id
    ));
    let body = merge_strategy.map(|s| serde_json::json!({ "merge_strategy": s }));
    let resp = client.request("POST", &url, None, body)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse merged PR: {}. Response: {}", e, resp))
}

pub fn decline_pr(
    client: &Client,
    workspace: &str,
    slug: &str,
    pr_id: i64,
) -> Result<PullRequest, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pullrequests/{}/decline",
        workspace, slug, pr_id
    ));
    let resp = client.request("POST", &url, None, None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse declined PR: {}. Response: {}", e, resp))
}

// ---------------------------------------------------------------------------
// Pipeline API
// ---------------------------------------------------------------------------

pub fn list_pipelines(
    client: &Client,
    workspace: &str,
    slug: &str,
    page_len: i32,
) -> Result<PipelinesPage, String> {
    let url = bb_url(&format!("/repositories/{}/{}/pipelines/", workspace, slug));
    let len_str = page_len.to_string();
    let q = [("pagelen", len_str.as_str()), ("sort", "-created_on")];
    let resp = client.request("GET", &url, Some(&q), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse pipelines: {}. Response: {}", e, resp))
}

pub fn get_pipeline(
    client: &Client,
    workspace: &str,
    slug: &str,
    uuid: &str,
) -> Result<Pipeline, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pipelines/{}",
        workspace, slug, uuid
    ));
    let resp = client.request("GET", &url, None, None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse pipeline: {}. Response: {}", e, resp))
}

pub fn run_pipeline(
    client: &Client,
    workspace: &str,
    slug: &str,
    branch: &str,
) -> Result<Pipeline, String> {
    let url = bb_url(&format!("/repositories/{}/{}/pipelines/", workspace, slug));
    let body = serde_json::json!({
        "target": {
            "ref_type": "branch",
            "type": "pipeline_ref_target",
            "ref_name": branch
        }
    });
    let resp = client.request("POST", &url, None, Some(body))?;
    serde_json::from_str(&resp).map_err(|e| {
        format!(
            "Failed to parse triggered pipeline: {}. Response: {}",
            e, resp
        )
    })
}

pub fn stop_pipeline(
    client: &Client,
    workspace: &str,
    slug: &str,
    uuid: &str,
) -> Result<(), String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pipelines/{}/stopPipeline",
        workspace, slug, uuid
    ));
    client.request("POST", &url, None, None)?;
    Ok(())
}

pub fn get_pipeline_log(
    client: &Client,
    workspace: &str,
    slug: &str,
    pipeline_uuid: &str,
    step_uuid: &str,
) -> Result<String, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pipelines/{}/steps/{}/log",
        workspace, slug, pipeline_uuid, step_uuid
    ));
    client.request("GET", &url, None, None)
}

// ---------------------------------------------------------------------------
// Pipeline step types + list steps
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipelineStep {
    pub uuid: String,
    pub name: Option<String>,
    pub state: PipelineState,
    #[serde(rename = "started_on")]
    pub started_on: Option<String>,
    #[serde(rename = "completed_on")]
    pub completed_on: Option<String>,
    #[serde(rename = "duration_in_seconds")]
    pub duration_in_seconds: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StepsPage {
    pub values: Vec<PipelineStep>,
}

pub fn list_pipeline_steps(
    client: &Client,
    workspace: &str,
    slug: &str,
    pipeline_uuid: &str,
) -> Result<StepsPage, String> {
    let url = bb_url(&format!(
        "/repositories/{}/{}/pipelines/{}/steps/",
        workspace, slug, pipeline_uuid
    ));
    let resp = client.request("GET", &url, None, None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse pipeline steps: {}. Response: {}", e, resp))
}
