use crate::client::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlertResponder {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub responder_type: Option<String>,
}

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
    // fields present in real JSM API responses
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    pub entity: Option<String>,
    pub source: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub snoozed: Option<bool>,
    #[serde(rename = "lastOccuredAt")]
    pub last_occured_at: Option<String>,
    #[serde(rename = "integrationType")]
    pub integration_type: Option<String>,
    #[serde(rename = "integrationName")]
    pub integration_name: Option<String>,
    pub owner: Option<String>,
    pub seen: Option<bool>,
    pub count: Option<u64>,
    pub description: Option<String>,
    #[serde(rename = "ackTime")]
    pub ack_time: Option<String>,
    #[serde(default)]
    pub responders: Vec<AlertResponder>,
    #[serde(default)]
    pub actions: Vec<String>,
}

/// List response: real JSM API wraps results in `values`, not `data`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlertListResponse {
    pub values: Vec<Alert>,
    pub count: Option<u64>,
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
    // Opsgenie query language: status values are `open` and `closed`.
    // "acknowledged" / "acked" are not status values — they map to `acknowledged:true`.
    let status_query = query_status.map(|s| match s {
        "acknowledged" | "acked" => "acknowledged:true".to_string(),
        "unacknowledged" => "acknowledged:false".to_string(),
        other => format!("status:{}", other),
    });
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(ref sq) = status_query {
        query.push(("query", sq.as_str()));
    }

    let resp = client.request_jsm("GET", "/alerts", Some(&query), None)?;
    let list_res: AlertListResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse alerts list: {}. Response: {}", e, resp))?;

    Ok(list_res.values)
}

/// Infers the Opsgenie `identifierType` from the identifier string.
/// All-digit strings are tinyIds; everything else is treated as a full id.
pub fn infer_id_type(identifier: &str) -> &'static str {
    if identifier.chars().all(|c| c.is_ascii_digit()) {
        "tinyId"
    } else {
        "id"
    }
}

pub fn get_alert(client: &Client, identifier: &str, id_type: &str) -> Result<Alert, String> {
    if id_type == "tinyId" {
        // Atlassian JSM cloud does not honour identifierType=tinyId on the GET endpoint.
        // Resolve by scanning the list for a matching tinyId field instead.
        let all = list_alerts(client, None)?;
        return all
            .into_iter()
            .find(|a| a.tiny_id.as_deref() == Some(identifier))
            .ok_or_else(|| format!("No alert found with tinyId: {}", identifier));
    }

    let path = format!("/alerts/{}", identifier);
    let query = [("identifierType", id_type)];

    let resp = client.request_jsm("GET", &path, Some(&query), None)?;
    // Real Opsgenie single-alert GET returns the Alert object directly (no wrapper).
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse alert: {}. Response: {}", e, resp))
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
