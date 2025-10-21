//! Tests for Publisher and Subscriber configurations

use sparkplug_rs::{PublisherConfig, SubscriberConfig};

#[test]
fn test_publisher_config_creation() {
    let config = PublisherConfig::new(
        "tcp://localhost:1883",
        "test_client",
        "TestGroup",
        "TestNode",
    );

    assert_eq!(config.broker_url, "tcp://localhost:1883");
    assert_eq!(config.client_id, "test_client");
    assert_eq!(config.group_id, "TestGroup");
    assert_eq!(config.edge_node_id, "TestNode");
}

#[test]
fn test_publisher_config_with_owned_strings() {
    let broker = String::from("tcp://broker:1883");
    let client = String::from("client1");
    let group = String::from("Group1");
    let node = String::from("Node1");

    let config = PublisherConfig::new(broker, client, group, node);

    assert_eq!(config.broker_url, "tcp://broker:1883");
    assert_eq!(config.client_id, "client1");
}

#[test]
fn test_publisher_config_clone() {
    let config1 = PublisherConfig::new(
        "tcp://localhost:1883",
        "client",
        "group",
        "node",
    );

    let config2 = config1.clone();

    assert_eq!(config1.broker_url, config2.broker_url);
    assert_eq!(config1.client_id, config2.client_id);
    assert_eq!(config1.group_id, config2.group_id);
    assert_eq!(config1.edge_node_id, config2.edge_node_id);
}

#[test]
fn test_subscriber_config_creation() {
    let config = SubscriberConfig::new(
        "tcp://localhost:1883",
        "sub_client",
        "TestGroup",
    );

    assert_eq!(config.broker_url, "tcp://localhost:1883");
    assert_eq!(config.client_id, "sub_client");
    assert_eq!(config.group_id, "TestGroup");
}

#[test]
fn test_subscriber_config_clone() {
    let config1 = SubscriberConfig::new(
        "tcp://localhost:1883",
        "client",
        "group",
    );

    let config2 = config1.clone();

    assert_eq!(config1.broker_url, config2.broker_url);
    assert_eq!(config1.client_id, config2.client_id);
    assert_eq!(config1.group_id, config2.group_id);
}

#[test]
fn test_config_with_special_characters() {
    let config = PublisherConfig::new(
        "ssl://broker.example.com:8883",
        "client-123_ABC",
        "Group/SubGroup",
        "Node#1",
    );

    assert_eq!(config.broker_url, "ssl://broker.example.com:8883");
    assert_eq!(config.client_id, "client-123_ABC");
    assert_eq!(config.group_id, "Group/SubGroup");
    assert_eq!(config.edge_node_id, "Node#1");
}
