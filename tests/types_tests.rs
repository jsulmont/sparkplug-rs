//! Tests for type conversions and data types

use sparkplug_rs::{DataType, MetricValue};

#[test]
fn test_datatype_enum_values() {
    // Verify DataType enum covers all expected types
    let types = vec![
        DataType::Unknown,
        DataType::Int8,
        DataType::Int16,
        DataType::Int32,
        DataType::Int64,
        DataType::UInt8,
        DataType::UInt16,
        DataType::UInt32,
        DataType::UInt64,
        DataType::Float,
        DataType::Double,
        DataType::Boolean,
        DataType::String,
        DataType::DateTime,
        DataType::Text,
    ];

    assert_eq!(types.len(), 15, "Should have all 15 data types");
}

#[test]
fn test_metric_value_variants() {
    // Test MetricValue variants
    let values = vec![
        MetricValue::Int8(42),
        MetricValue::Int16(1234),
        MetricValue::Int32(123456),
        MetricValue::Int64(123456789),
        MetricValue::UInt8(255),
        MetricValue::UInt16(65535),
        MetricValue::UInt32(4294967295),
        MetricValue::UInt64(18446744073709551615),
        MetricValue::Float(3.14),
        MetricValue::Double(2.71828),
        MetricValue::Boolean(true),
        MetricValue::String("test".to_string()),
        MetricValue::Null,
    ];

    assert_eq!(values.len(), 13, "Should have all MetricValue variants");
}

#[test]
fn test_metric_value_equality() {
    assert_eq!(MetricValue::Int32(42), MetricValue::Int32(42));
    assert_ne!(MetricValue::Int32(42), MetricValue::Int32(43));

    assert_eq!(
        MetricValue::String("test".to_string()),
        MetricValue::String("test".to_string())
    );

    assert_eq!(MetricValue::Null, MetricValue::Null);
}

#[test]
fn test_metric_value_clone() {
    let value = MetricValue::Double(3.14);
    let cloned = value.clone();
    assert_eq!(value, cloned);

    let string_value = MetricValue::String("test".to_string());
    let cloned_string = string_value.clone();
    assert_eq!(string_value, cloned_string);
}

#[test]
fn test_datatype_copy() {
    let dt1 = DataType::Double;
    let dt2 = dt1; // Should copy, not move
    assert_eq!(dt1, dt2);
    assert_eq!(dt1, DataType::Double); // dt1 still usable
}
