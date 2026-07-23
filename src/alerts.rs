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
pub struct AlertNote {
    pub note: String,
    pub owner: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlertNotesResponse {
    pub values: Vec<AlertNote>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlertLog {
    #[serde(rename = "logTime")]
    pub log_time: Option<String>,
    #[serde(rename = "logType")]
    pub log_type: Option<String>,
    pub log: String,
    pub owner: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlertLogsResponse {
    pub values: Vec<AlertLog>,
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

/// Resolves a tinyId to a full UUID by querying the list endpoint.
fn resolve_tiny_id(client: &Client, tiny_id: &str) -> Result<String, String> {
    let tinyid_query = format!("tinyId:{}", tiny_id);
    let query = [("query", tinyid_query.as_str()), ("limit", "1")];
    let resp = client.request_jsm("GET", "/alerts", Some(&query), None)?;
    let list_res: AlertListResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to resolve tinyId: {}. Response: {}", e, resp))?;
    list_res
        .values
        .into_iter()
        .next()
        .map(|a| a.id)
        .ok_or_else(|| format!("No alert found with tinyId: {}", tiny_id))
}

pub fn get_alert(client: &Client, identifier: &str, id_type: &str) -> Result<Alert, String> {
    // Atlassian JSM cloud does not honour identifierType=tinyId on the GET endpoint.
    // Resolve to a full UUID first, then fetch the detail endpoint for complete fields.
    let resolved_id = if id_type == "tinyId" {
        resolve_tiny_id(client, identifier)?
    } else {
        identifier.to_string()
    };

    let path = format!("/alerts/{}", resolved_id);
    let query = [("identifierType", "id")];

    let resp = client.request_jsm("GET", &path, Some(&query), None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse alert: {}. Response: {}", e, resp))
}

pub fn list_alert_notes(client: &Client, alert_id: &str) -> Result<Vec<AlertNote>, String> {
    let path = format!("/alerts/{}/notes", alert_id);
    let query = [("identifierType", "id"), ("limit", "100"), ("order", "asc")];

    let resp = client.request_jsm("GET", &path, Some(&query), None)?;
    let res: AlertNotesResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse alert notes: {}. Response: {}", e, resp))?;

    Ok(res.values)
}

pub fn list_alert_logs(client: &Client, alert_id: &str) -> Result<Vec<AlertLog>, String> {
    let path = format!("/alerts/{}/logs", alert_id);
    let query = [("identifierType", "id"), ("limit", "100"), ("order", "asc")];

    let resp = client.request_jsm("GET", &path, Some(&query), None)?;
    let res: AlertLogsResponse = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse alert logs: {}. Response: {}", e, resp))?;

    Ok(res.values)
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnCallResponse {
    #[serde(rename = "onCallUsers")]
    pub on_call_users: Vec<String>,
}

pub fn get_oncall(client: &Client, schedule_id: Option<&str>) -> Result<Vec<String>, String> {
    // JSM API requires schedule ID, not name. If none provided, return error.
    let sid = schedule_id.ok_or("Schedule ID is required. Use: acli alert oncall <schedule-id>")?;

    let query = [("flat", "true")];
    let path = format!("/schedules/{}/on-calls", sid);

    let resp = client.request_jsm("GET", &path, Some(&query), None)?;
    let oncall_res: OnCallResponse = serde_json::from_str(&resp).map_err(|e| {
        format!(
            "Failed to parse on-call response: {}. Response: {}",
            e, resp
        )
    })?;
    Ok(oncall_res.on_call_users)
}

/// Parses an ISO-8601 UTC timestamp (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SS[Z]`)
/// into seconds since the Unix epoch. Only UTC (`Z` or no offset) is supported.
fn parse_iso8601_utc(s: &str) -> Result<i64, String> {
    let err = || format!("Invalid date/time '{}'. Expected format: YYYY-MM-DDTHH:MM:SSZ", s);

    let (date_part, time_part) = match s.split_once('T') {
        Some((d, t)) => (d, t.trim_end_matches('Z')),
        None => (s, "00:00:00"),
    };

    let date_fields: Vec<&str> = date_part.split('-').collect();
    if date_fields.len() != 3 {
        return Err(err());
    }
    let year: i64 = date_fields[0].parse().map_err(|_| err())?;
    let month: i64 = date_fields[1].parse().map_err(|_| err())?;
    let day: i64 = date_fields[2].parse().map_err(|_| err())?;

    let time_fields: Vec<&str> = time_part.split(':').collect();
    if time_fields.is_empty() || time_fields.len() > 3 {
        return Err(err());
    }
    let hour: i64 = time_fields.first().map_or(Ok(0), |v| v.parse()).map_err(|_| err())?;
    let minute: i64 = time_fields.get(1).map_or(Ok(0), |v| v.parse()).map_err(|_| err())?;
    let second: i64 = time_fields
        .get(2)
        .map_or(Ok(0.0), |v| v.parse::<f64>())
        .map_err(|_| err())? as i64;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(err());
    }

    // Howard Hinnant's days_from_civil algorithm.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;

    Ok(days * 86400 + hour * 3600 + minute * 60 + second)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimelineResponder {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub responder_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimelinePeriod {
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,
    #[serde(rename = "type")]
    pub period_type: Option<String>,
    pub responder: Option<TimelineResponder>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TimelineRotation {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub periods: Vec<TimelinePeriod>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FinalTimeline {
    #[serde(default)]
    pub rotations: Vec<TimelineRotation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimelineResponse {
    #[serde(rename = "finalTimeline")]
    pub final_timeline: FinalTimeline,
}

/// Returns the on-call periods for a single schedule that overlap `[from, until)`.
pub fn get_oncall_timeline(
    client: &Client,
    schedule_id: &str,
    from: &str,
    until: &str,
) -> Result<Vec<TimelinePeriod>, String> {
    let from_secs = parse_iso8601_utc(from)?;
    let until_secs = parse_iso8601_utc(until)?;
    if until_secs <= from_secs {
        return Err("`until` must be after `from`".to_string());
    }

    let interval_days = ((until_secs - from_secs) + 86399) / 86400; // ceil to whole days
    let interval_str = interval_days.max(1).to_string();
    let query = [
        ("date", from),
        ("interval", interval_str.as_str()),
        ("intervalUnit", "days"),
    ];

    let path = format!("/schedules/{}/timeline", schedule_id);
    let resp = client.request_jsm("GET", &path, Some(&query), None)?;
    let timeline: TimelineResponse = serde_json::from_str(&resp).map_err(|e| {
        format!(
            "Failed to parse schedule timeline: {}. Response: {}",
            e, resp
        )
    })?;

    let mut periods: Vec<TimelinePeriod> = timeline
        .final_timeline
        .rotations
        .into_iter()
        .flat_map(|r| r.periods.into_iter())
        .filter(|p| {
            let p_start = parse_iso8601_utc(&p.start_date).unwrap_or(i64::MAX);
            let p_end = parse_iso8601_utc(&p.end_date).unwrap_or(i64::MIN);
            p_start < until_secs && p_end > from_secs
        })
        .collect();

    periods.sort_by(|a, b| a.start_date.cmp(&b.start_date));
    Ok(periods)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleOnCall {
    #[serde(rename = "scheduleId")]
    pub schedule_id: String,
    #[serde(rename = "scheduleName")]
    pub schedule_name: String,
    pub periods: Vec<TimelinePeriod>,
    pub error: Option<String>,
}

/// Returns on-call periods across multiple schedules for `[from, until)`.
/// A failure on one schedule is recorded in `ScheduleOnCall.error` rather than
/// aborting the whole lookup, so one broken schedule doesn't block the rest.
pub fn get_oncall_timeline_for_schedules(
    client: &Client,
    schedules: &[Schedule],
    from: &str,
    until: &str,
) -> Vec<ScheduleOnCall> {
    schedules
        .iter()
        .map(
            |s| match get_oncall_timeline(client, &s.id, from, until) {
                Ok(periods) => ScheduleOnCall {
                    schedule_id: s.id.clone(),
                    schedule_name: s.name.clone(),
                    periods,
                    error: None,
                },
                Err(e) => ScheduleOnCall {
                    schedule_id: s.id.clone(),
                    schedule_name: s.name.clone(),
                    periods: Vec::new(),
                    error: Some(e),
                },
            },
        )
        .collect()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Team {
    #[serde(rename = "teamId")]
    pub id: String,
    #[serde(rename = "teamName")]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub fn list_teams(client: &Client) -> Result<Vec<Team>, String> {
    let resp = client.request_jsm("GET", "/teams", None, None)?;
    let teams: Vec<Team> = serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse teams response: {}. Response: {}", e, resp))?;
    Ok(teams)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Schedule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "teamId")]
    pub team_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchedulesResponse {
    pub values: Vec<Schedule>,
}

pub fn list_schedules(
    client: &Client,
    escalation_schedules: Option<Vec<crate::config::EscalationSchedule>>,
) -> Result<Vec<Schedule>, String> {
    let resp = client.request_jsm("GET", "/schedules", None, None)?;
    let schedules_res: SchedulesResponse = serde_json::from_str(&resp).map_err(|e| {
        format!(
            "Failed to parse schedules response: {}. Response: {}",
            e, resp
        )
    })?;

    let mut all_schedules = schedules_res.values;

    if let Some(escalations) = escalation_schedules {
        for esc in &escalations {
            if !all_schedules.iter().any(|s| s.id == esc.schedule_id) {
                all_schedules.push(Schedule {
                    id: esc.schedule_id.clone(),
                    name: format!("{} (escalation)", esc.name),
                    description: Some("Configured escalation schedule".to_string()),
                    team_id: None,
                });
            }
        }
    }

    Ok(all_schedules)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    #[serde(rename = "accountId")]
    pub account_id: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
    pub active: Option<bool>,
    #[serde(rename = "accountType")]
    pub account_type: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
    // Opsgenie fields (kept for backward compatibility)
    pub id: Option<String>,
    #[serde(rename = "username")]
    pub username: Option<String>,
    #[serde(rename = "fullName")]
    pub full_name: Option<String>,
}

/// Looks up a user directly via the Jira REST API by account ID.
/// Schedule timeline responders are always Atlassian account IDs (with or
/// without the `<siteId>:` prefix), so this bypasses `get_user`'s heuristic.
pub fn get_jira_user(client: &Client, account_id: &str) -> Result<User, String> {
    let path = format!("/rest/api/3/user?accountId={}", account_id);
    let resp = client.request("GET", &path, None, None)?;
    serde_json::from_str(&resp)
        .map_err(|e| format!("Failed to parse Jira user: {}. Response: {}", e, resp))
}

/// Look up a user by ID. Auto-detects between Jira account IDs (contain `:`)
/// and Opsgenie user IDs (plain UUIDs).
pub fn get_user(client: &Client, user_id: &str) -> Result<User, String> {
    if user_id.contains(':') {
        // Jira account ID → use Jira REST API
        let path = format!("/rest/api/3/user?accountId={}", user_id);
        let resp = client.request("GET", &path, None, None)?;
        serde_json::from_str(&resp)
            .map_err(|e| format!("Failed to parse Jira user: {}. Response: {}", e, resp))
    } else {
        // Opsgenie user ID → use JSM Ops API
        let path = format!("/users/{}", user_id);
        let query = [("identifierType", "id")];
        let resp = client.request_jsm("GET", &path, Some(&query), None)?;
        serde_json::from_str(&resp)
            .map_err(|e| format!("Failed to parse user: {}. Response: {}", e, resp))
    }
}
