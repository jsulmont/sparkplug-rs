//! Tests for PayloadBuilder and Payload parsing

use sparkplug_rs::{Error, PayloadBuilder};

#[test]
fn test_payload_builder_creation() {
    let builder = PayloadBuilder::new();
    assert!(builder.is_ok(), "Should create PayloadBuilder successfully");
}

#[test]
fn test_add_metrics_by_name() {
    let mut builder = PayloadBuilder::new().unwrap();

    // Test all numeric types
    assert!(builder.add_int8("test_i8", 42).is_ok());
    assert!(builder.add_int16("test_i16", 1234).is_ok());
    assert!(builder.add_int32("test_i32", 123456).is_ok());
    assert!(builder.add_int64("test_i64", 123456789).is_ok());
    assert!(builder.add_uint8("test_u8", 255).is_ok());
    assert!(builder.add_uint16("test_u16", 65535).is_ok());
    assert!(builder.add_uint32("test_u32", 4294967295).is_ok());
    assert!(builder.add_uint64("test_u64", 18446744073709551615).is_ok());
    assert!(builder.add_float("test_f32", std::f32::consts::PI).is_ok());
    assert!(builder.add_double("test_f64", std::f64::consts::E).is_ok());
    assert!(builder.add_bool("test_bool", true).is_ok());
    assert!(builder.add_string("test_str", "hello").is_ok());
}

#[test]
fn test_null_byte_in_metric_name() {
    let mut builder = PayloadBuilder::new().unwrap();

    // Metric name with null byte should fail
    let result = builder.add_int32("test\0name", 123);
    assert!(result.is_err(), "Should reject null bytes in metric name");

    match result {
        Err(Error::NulError(_)) => {
            // Expected error type
        }
        _ => panic!("Expected NulError"),
    }
}

#[test]
fn test_null_byte_in_string_value() {
    let mut builder = PayloadBuilder::new().unwrap();

    // String value with null byte should fail
    let result = builder.add_string("test", "hello\0world");
    assert!(result.is_err(), "Should reject null bytes in string value");
}

#[test]
fn test_builder_method_chaining() {
    let mut builder = PayloadBuilder::new().unwrap();

    let result = builder
        .add_int32("metric1", 100)
        .and_then(|b| b.add_double("metric2", std::f64::consts::PI))
        .and_then(|b| b.add_bool("metric3", true));

    assert!(result.is_ok(), "Method chaining should work");
}

#[test]
fn test_metrics_with_aliases() {
    let mut builder = PayloadBuilder::new().unwrap();

    assert!(builder.add_int32_with_alias("temp", 1, 25).is_ok());
    assert!(builder.add_int64_with_alias("uptime", 2, 12345).is_ok());
    assert!(builder.add_uint32_with_alias("count", 3, 999).is_ok());
    assert!(builder.add_uint64_with_alias("total", 4, 1000000).is_ok());
    assert!(builder.add_float_with_alias("voltage", 5, 3.3).is_ok());
    assert!(builder.add_double_with_alias("current", 6, 1.5).is_ok());
    assert!(builder.add_bool_with_alias("active", 7, false).is_ok());
}

#[test]
fn test_serialize_empty_payload() {
    let builder = PayloadBuilder::new().unwrap();
    let bytes = builder.serialize();

    assert!(bytes.is_ok(), "Should serialize empty payload");
    let bytes = bytes.unwrap();
    assert!(!bytes.is_empty(), "Serialized payload should not be empty");
}

#[test]
fn test_serialize_with_metrics() {
    let mut builder = PayloadBuilder::new().unwrap();
    builder
        .add_double("Temperature", 20.5)
        .unwrap()
        .add_bool("Active", true)
        .unwrap();

    let bytes = builder.serialize();
    assert!(bytes.is_ok(), "Should serialize payload with metrics");
    let bytes = bytes.unwrap();
    assert!(!bytes.is_empty(), "Serialized payload should contain data");
}

#[test]
fn test_payload_round_trip() {
    use sparkplug_rs::Payload;

    // Build a payload
    let mut builder = PayloadBuilder::new().unwrap();
    builder
        .add_int32("metric1", 42)
        .unwrap()
        .add_double("metric2", std::f64::consts::PI)
        .unwrap()
        .add_bool("metric3", true)
        .unwrap()
        .add_string("metric4", "test")
        .unwrap();

    let bytes = builder.serialize().unwrap();

    // Parse it back
    let payload = Payload::parse(&bytes);
    assert!(payload.is_ok(), "Should parse serialized payload");

    let payload = payload.unwrap();
    assert_eq!(payload.metric_count(), 4, "Should have 4 metrics");
}

#[test]
fn test_payload_parse_invalid_data() {
    use sparkplug_rs::Payload;

    let invalid_data = vec![0xFF, 0xFF, 0xFF, 0xFF];
    let result = Payload::parse(&invalid_data);

    assert!(result.is_err(), "Should fail to parse invalid data");
    match result {
        Err(Error::ParseFailed) => {
            // Expected error type
        }
        _ => panic!("Expected ParseFailed"),
    }
}

#[test]
fn test_payload_metric_iteration() {
    use sparkplug_rs::Payload;

    let mut builder = PayloadBuilder::new().unwrap();
    builder
        .add_int32("m1", 1)
        .unwrap()
        .add_int32("m2", 2)
        .unwrap()
        .add_int32("m3", 3)
        .unwrap();

    let bytes = builder.serialize().unwrap();
    let payload = Payload::parse(&bytes).unwrap();

    let metrics: Vec<_> = payload.metrics().collect();
    assert_eq!(metrics.len(), 3, "Should have 3 metrics");

    for metric in metrics {
        assert!(metric.is_ok(), "Each metric should parse successfully");
    }
}

#[test]
fn test_payload_invalid_index() {
    use sparkplug_rs::Payload;

    let builder = PayloadBuilder::new().unwrap();
    let bytes = builder.serialize().unwrap();
    let payload = Payload::parse(&bytes).unwrap();

    // Empty payload, index 0 should be invalid
    let result = payload.metric_at(0);
    assert!(result.is_err(), "Should fail for invalid index");

    if let Err(Error::InvalidMetricIndex { index, count }) = result {
        assert_eq!(index, 0);
        assert_eq!(count, 0);
    } else {
        panic!("Expected InvalidMetricIndex, got {:?}", result);
    }
}

#[test]
fn test_timestamp_and_seq() {
    use sparkplug_rs::Payload;

    let mut builder = PayloadBuilder::new().unwrap();
    builder.set_timestamp(1234567890);
    builder.set_seq(42);

    let bytes = builder.serialize().unwrap();
    let payload = Payload::parse(&bytes).unwrap();

    assert_eq!(payload.timestamp(), Some(1234567890));
    assert_eq!(payload.seq(), Some(42));
}

#[test]
fn test_metrics_by_alias() {
    let mut builder = PayloadBuilder::new().unwrap();

    // Add by alias only (for NDATA messages)
    builder.add_int32_by_alias(1, 100);
    builder.add_int64_by_alias(2, 200);
    builder.add_uint32_by_alias(3, 300);
    builder.add_uint64_by_alias(4, 400);
    builder.add_float_by_alias(5, 5.0);
    builder.add_double_by_alias(6, 6.0);
    builder.add_bool_by_alias(7, true);

    let bytes = builder.serialize();
    assert!(bytes.is_ok(), "Should serialize alias-only metrics");
}

#[test]
fn test_large_values() {
    let mut builder = PayloadBuilder::new().unwrap();

    // Test boundary values
    builder.add_int8("i8_min", i8::MIN).unwrap();
    builder.add_int8("i8_max", i8::MAX).unwrap();
    builder.add_int64("i64_min", i64::MIN).unwrap();
    builder.add_int64("i64_max", i64::MAX).unwrap();
    builder.add_uint64("u64_max", u64::MAX).unwrap();

    let bytes = builder.serialize();
    assert!(bytes.is_ok(), "Should handle boundary values");
}

#[test]
fn test_unicode_strings() {
    let mut builder = PayloadBuilder::new().unwrap();

    // Unicode in metric names
    assert!(builder.add_string("温度", "value1").is_ok());
    assert!(builder.add_string("name", "Hello 世界 🌍").is_ok());

    let bytes = builder.serialize();
    assert!(bytes.is_ok(), "Should handle Unicode strings");
}
