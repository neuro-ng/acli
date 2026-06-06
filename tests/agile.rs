mod common;

use acli_rust::agile;
use acli_rust::client::Client;
use common::{mock_profile, start_mock_jira_write_server};

#[test]
fn test_agile_list_boards() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let boards = agile::list_boards(&client, 0, 50, None).unwrap();
    assert_eq!(boards.total, 1);
    assert_eq!(boards.values[0].id, 42);
    assert_eq!(boards.values[0].name, "Rust Board");
    assert_eq!(boards.values[0].board_type, "scrum");
    assert_eq!(
        boards.values[0].location.as_ref().unwrap().project_key,
        "TEST"
    );
}

#[test]
fn test_agile_get_board_sprints() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let sprints = agile::get_board_sprints(&client, 42, 0, 50).unwrap();
    assert_eq!(sprints.total, 2);
    assert_eq!(sprints.values[0].id, 101);
    assert_eq!(sprints.values[0].name, "Sprint 1");
    assert_eq!(sprints.values[0].state, "active");
    assert_eq!(sprints.values[1].id, 102);
    assert_eq!(sprints.values[1].state, "future");
}

#[test]
fn test_agile_get_sprint_issues() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let results = agile::get_sprint_issues(&client, 101, 0, 50).unwrap();
    assert_eq!(results.total, Some(1));
    assert_eq!(results.issues[0].key, "TEST-201");
    assert_eq!(results.issues[0].fields.summary, "Sprint task");
}

#[test]
fn test_agile_get_epic_issues() {
    let url = start_mock_jira_write_server();
    let client = Client::new(mock_profile(&url));

    let results = agile::get_epic_issues(&client, "EPIC-1", 0, 50).unwrap();
    assert_eq!(results.total, Some(1));
    assert_eq!(results.issues[0].key, "TEST-301");
    assert_eq!(results.issues[0].fields.summary, "Epic child issue");
}
