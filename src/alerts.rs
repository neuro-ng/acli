use crate::client::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Alert {
    pub id: String,
    #[serde(rename = "tinyId")]
    pub tiny_id: Option<String>,
    pub alias: Option<String>,
    pub message: String,
    pub status: String,
    pub acknowledged: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub priority: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlertListResponse {
    pub data: Vec<Alert>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlertResponse {
    pub data: Alert,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateAlertPayload {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

pub fn create_alert(client: &Client, payload: CreateAlertPayload) -> Result<String, String> {
    let body = serde_json::to_value(&payload)
        .map_err(|e| format!("Failed to serialize create alert payload: {}", e))?;

    let resp = client.request_jsm("POST", "/alerts", None, Some(body))?;
    Ok(resp)
}

pub fn list_alerts(client: &Client, query_status: Option<&str>) -> Result<Vec<Alert>, String> {
    let mut query = Vec::new();
    if let Some(status) = query_status {
        query.push(("query", status));
    }

    let resp = client.request_jsm("GET", "/alerts", Some(&query), None)?;
    let list_res: AlertListResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse alerts list: {}. Response: {}", e, resp))?;

    Ok(list_res.data)
}

pub fn get_alert(client: &Client, identifier: &str, id_type: &str) -> Result<Alert, String> {
    let path = format!("/alerts/{}", identifier);
    let query = [("identifierType", id_type)];

    let resp = client.request_jsm("GET", &path, Some(&query), None)?;
    let res: AlertResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse alert: {}. Response: {}", e, resp))?;

    Ok(res.data)
}

pub fn acknowledge_alert(
    client: &Client,
    identifier: &str,
    id_type: &str,
    note: Option<&str>,
) -> Result<String, String> {
    let path = format!("/alerts/{}/acknowledge", identifier);
    let query = [("identifierType", id_type)];

    let mut body_map = serde_json::Map::new();
    if let Some(n) = note {
        body_map.insert("note".to_string(), serde_json::Value::String(n.to_string()));
    }
    let body = serde_json::Value::Object(body_map);

    let resp = client.request_jsm("POST", &path, Some(&query), Some(body))?;
    Ok(resp)
}

pub fn close_alert(
    client: &Client,
    identifier: &str,
    id_type: &str,
    note: Option<&str>,
) -> Result<String, String> {
    let path = format!("/alerts/{}/close", identifier);
    let query = [("identifierType", id_type)];

    let mut body_map = serde_json::Map::new();
    if let Some(n) = note {
        body_map.insert("note".to_string(), serde_json::Value::String(n.to_string()));
    }
    let body = serde_json::Value::Object(body_map);

    let resp = client.request_jsm("POST", &path, Some(&query), Some(body))?;
    Ok(resp)
}
