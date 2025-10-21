//! Tests for topic parsing

use sparkplug_rs::{MessageType, ParsedTopic};

#[test]
fn test_parse_nbirth_topic() {
    let topic = ParsedTopic::parse("spBv1.0/Energy/NBIRTH/Gateway01").unwrap();
    assert_eq!(topic.message_type(), Some(MessageType::NBirth));
    assert_eq!(topic.group_id(), Some("Energy"));
    assert_eq!(topic.edge_node_id(), Some("Gateway01"));
    assert_eq!(topic.device_id(), None);
}

#[test]
fn test_parse_ndeath_topic() {
    let topic = ParsedTopic::parse("spBv1.0/Manufacturing/NDEATH/Node1").unwrap();
    assert_eq!(topic.message_type(), Some(MessageType::NDeath));
    assert_eq!(topic.group_id(), Some("Manufacturing"));
    assert_eq!(topic.edge_node_id(), Some("Node1"));
}

#[test]
fn test_parse_ndata_topic() {
    let topic = ParsedTopic::parse("spBv1.0/Production/NDATA/EdgeNode01").unwrap();
    assert_eq!(topic.message_type(), Some(MessageType::NData));
    assert_eq!(topic.group_id(), Some("Production"));
}

#[test]
fn test_parse_ncmd_topic() {
    let topic = ParsedTopic::parse("spBv1.0/Energy/NCMD/Gateway01").unwrap();
    assert_eq!(topic.message_type(), Some(MessageType::NCmd));
}

#[test]
fn test_parse_dbirth_topic() {
    let topic = ParsedTopic::parse("spBv1.0/Energy/DBIRTH/Gateway01/Sensor01").unwrap();
    assert_eq!(topic.message_type(), Some(MessageType::DBirth));
    assert_eq!(topic.group_id(), Some("Energy"));
    assert_eq!(topic.edge_node_id(), Some("Gateway01"));
    assert_eq!(topic.device_id(), Some("Sensor01"));
}

#[test]
fn test_parse_ddeath_topic() {
    let topic = ParsedTopic::parse("spBv1.0/Factory/DDEATH/Node1/Device1").unwrap();
    assert_eq!(topic.message_type(), Some(MessageType::DDeath));
    assert_eq!(topic.device_id(), Some("Device1"));
}

#[test]
fn test_parse_ddata_topic() {
    let topic = ParsedTopic::parse("spBv1.0/Plant/DDATA/Gateway/Sensor").unwrap();
    assert_eq!(topic.message_type(), Some(MessageType::DData));
    assert_eq!(topic.device_id(), Some("Sensor"));
}

#[test]
fn test_parse_dcmd_topic() {
    let topic = ParsedTopic::parse("spBv1.0/Control/DCMD/Node1/Actuator1").unwrap();
    assert_eq!(topic.message_type(), Some(MessageType::DCmd));
}

#[test]
fn test_parse_state_topic() {
    let topic = ParsedTopic::parse("STATE/ScadaHost01").unwrap();
    assert_eq!(topic.message_type(), None);
    assert_eq!(topic.host_id(), Some("ScadaHost01"));
    assert_eq!(topic.group_id(), None);
    assert_eq!(topic.edge_node_id(), None);
}

#[test]
fn test_invalid_prefix() {
    let result = ParsedTopic::parse("invalid/Energy/NDATA/Node1");
    assert!(result.is_err());
}

#[test]
fn test_too_few_parts() {
    let result = ParsedTopic::parse("spBv1.0/Energy/NDATA");
    assert!(result.is_err());
}

#[test]
fn test_unknown_message_type() {
    let result = ParsedTopic::parse("spBv1.0/Energy/UNKNOWN/Node1");
    assert!(result.is_err());
}

#[test]
fn test_device_message_without_device_id() {
    // DBIRTH requires a device_id
    let result = ParsedTopic::parse("spBv1.0/Energy/DBIRTH/Node1");
    assert!(result.is_err());
}

#[test]
fn test_node_message_with_device_id() {
    // NDATA should not have a device_id
    let result = ParsedTopic::parse("spBv1.0/Energy/NDATA/Node1/Device1");
    assert!(result.is_err());
}

#[test]
fn test_to_topic_string_node() {
    let original = "spBv1.0/Energy/NDATA/Gateway01";
    let topic = ParsedTopic::parse(original).unwrap();
    assert_eq!(topic.to_topic_string(), original);
}

#[test]
fn test_to_topic_string_device() {
    let original = "spBv1.0/Manufacturing/DDATA/Node1/Sensor01";
    let topic = ParsedTopic::parse(original).unwrap();
    assert_eq!(topic.to_topic_string(), original);
}

#[test]
fn test_to_topic_string_state() {
    let original = "STATE/ScadaHost01";
    let topic = ParsedTopic::parse(original).unwrap();
    assert_eq!(topic.to_topic_string(), original);
}

#[test]
fn test_message_type_predicates() {
    assert!(MessageType::NBirth.is_node_message());
    assert!(MessageType::NBirth.is_birth());
    assert!(!MessageType::NBirth.is_device_message());
    assert!(!MessageType::NBirth.is_death());

    assert!(MessageType::DBirth.is_device_message());
    assert!(MessageType::DBirth.is_birth());
    assert!(!MessageType::DBirth.is_node_message());

    assert!(MessageType::NData.is_data());
    assert!(MessageType::DData.is_data());

    assert!(MessageType::NCmd.is_command());
    assert!(MessageType::DCmd.is_command());

    assert!(MessageType::NDeath.is_death());
    assert!(MessageType::DDeath.is_death());
}

#[test]
fn test_message_type_display() {
    assert_eq!(MessageType::NBirth.to_string(), "NBIRTH");
    assert_eq!(MessageType::DData.to_string(), "DDATA");
    assert_eq!(MessageType::State.to_string(), "STATE");
}

#[test]
fn test_message_type_from_str() {
    use std::str::FromStr;

    assert_eq!(
        MessageType::from_str("NBIRTH").unwrap(),
        MessageType::NBirth
    );
    assert_eq!(MessageType::from_str("DDATA").unwrap(), MessageType::DData);
    assert_eq!(MessageType::from_str("STATE").unwrap(), MessageType::State);

    assert!(MessageType::from_str("INVALID").is_err());
}

#[test]
fn test_parsed_topic_display() {
    let topic = ParsedTopic::parse("spBv1.0/Energy/NDATA/Gateway01").unwrap();
    assert_eq!(topic.to_string(), "spBv1.0/Energy/NDATA/Gateway01");
}

#[test]
fn test_special_characters_in_ids() {
    let topic = ParsedTopic::parse("spBv1.0/Group-1/NDATA/Node_01").unwrap();
    assert_eq!(topic.group_id(), Some("Group-1"));
    assert_eq!(topic.edge_node_id(), Some("Node_01"));
}
