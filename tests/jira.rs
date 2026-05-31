mod common;

use acli_rust::client::Client;
use acli_rust::jira;
use common::{mock_profile, start_mock_atlassian_sdk, start_mock_jira_write_server};

#[test]
fn test_mock_jira_get_issue() {
    let url = start_mock_atlassian_sdk();
    let client = Client::new(mock_profile(&url));
    let issue = jira::get_issue(&client, "TEST-101").unwrap();
    assert_eq!(issue.key, "TEST-101");
    assert_eq!(issue.fields.summary, "Implement local mock SDK");
    assert_eq!(issue.fields.status.unwrap().name, "In Progress");
    assert_eq!(issue.fields.priority.unwrap().name, "High");
}

#[test]
fn test_mock_jira_search_jql() {
    let url = start_mock_atlassian_sdk();
    let client = Client::new(mock_profile(&url));
    let results = jira::search_jql(&client, "project = TEST", 0, 50, &["summary"]).unwrap();
    assert_eq!(results.total, 1);
    assert_eq!(results.issues[0].key, "TEST-101");
}

#[test]
fn test_jira_create_issue() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let created = jira::create_issue(&client, "TEST", "New task", "Task", None, None).unwrap();
    assert_eq!(created.key, "TEST-102");
    assert_eq!(created.id, "102");
}

#[test]
fn test_jira_edit_issue() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    // 204 No Content — edit returns nothing on success
    jira::edit_issue(&client, "TEST-101", Some("Updated summary"), None, None).unwrap();
}

#[test]
fn test_jira_delete_issue() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    jira::delete_issue(&client, "TEST-101", false).unwrap();
}

#[test]
fn test_jira_assign_issue() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    jira::assign_issue(&client, "TEST-101", Some("account-xyz")).unwrap();
}

#[test]
fn test_jira_get_transitions() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let transitions = jira::get_transitions(&client, "TEST-101").unwrap();
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].id, "21");
    assert_eq!(transitions[0].name, "In Progress");
}

#[test]
fn test_jira_do_transition() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    jira::do_transition(&client, "TEST-101", "31").unwrap();
}

#[test]
fn test_jira_list_comments() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let comments = jira::list_comments(&client, "TEST-101").unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].id, "comment-1");
    assert_eq!(comments[0].author.as_ref().unwrap().display_name, "Tester");
}

#[test]
fn test_jira_add_comment() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let comment = jira::add_comment(&client, "TEST-101", "Great work!").unwrap();
    assert_eq!(comment.id, "comment-1");
}

#[test]
fn test_jira_delete_comment() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    jira::delete_comment(&client, "TEST-101", "comment-1").unwrap();
}

#[test]
fn test_jira_list_worklogs() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let worklogs = jira::list_worklogs(&client, "TEST-101").unwrap();
    assert_eq!(worklogs.len(), 1);
    assert_eq!(worklogs[0].id, "worklog-1");
    assert_eq!(worklogs[0].time_spent.as_deref(), Some("2h"));
}

#[test]
fn test_jira_add_worklog() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let worklog = jira::add_worklog(&client, "TEST-101", "2h", Some("Fixed the bug")).unwrap();
    assert_eq!(worklog.id, "worklog-1");
    assert_eq!(worklog.time_spent.as_deref(), Some("2h"));
}

#[test]
fn test_jira_delete_worklog() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    jira::delete_worklog(&client, "TEST-101", "worklog-1").unwrap();
}
