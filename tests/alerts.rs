mod common;

use acli_rust::alerts;
use acli_rust::client::Client;
use common::{mock_profile, start_mock_atlassian_sdk};

#[test]
fn test_mock_jsm_alerts() {
    let url = start_mock_atlassian_sdk();
    let client = Client::new(mock_profile(&url));

    let alert_list = alerts::list_alerts(&client, None).unwrap();
    assert_eq!(alert_list.len(), 1);
    assert_eq!(alert_list[0].message, "Memory leak detected");

    let alert = alerts::get_alert(&client, "alert-uuid-1", "id").unwrap();
    assert_eq!(alert.id, "alert-uuid-1");
    assert_eq!(alert.tiny_id.as_deref(), Some("101"));

    let create_res = alerts::create_alert(
        &client,
        alerts::CreateAlertPayload {
            message: "Server down".to_string(),
            description: Some("Production server offline".to_string()),
            alias: Some("prod-down".to_string()),
            priority: Some("P1".to_string()),
        },
    )
    .unwrap();
    assert!(create_res.contains("req-create-123"));

    let ack_res =
        alerts::acknowledge_alert(&client, "alert-uuid-1", "id", Some("Ack note")).unwrap();
    assert!(ack_res.contains("req-ack-123"));

    let close_res = alerts::close_alert(&client, "alert-uuid-1", "id", Some("Close note")).unwrap();
    assert!(close_res.contains("req-close-123"));
}
