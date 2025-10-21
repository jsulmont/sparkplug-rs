//! Common types for the Sparkplug API.

use crate::sys;

/// A type-safe wrapper for Sparkplug metric aliases.
///
/// Aliases are used in birth certificates to establish a mapping between
/// metric names and numeric identifiers for bandwidth-efficient updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetricAlias(pub u64);

impl MetricAlias {
    /// Creates a new metric alias.
    pub const fn new(alias: u64) -> Self {
        Self(alias)
    }

    /// Gets the underlying u64 value.
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl From<u64> for MetricAlias {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<MetricAlias> for u64 {
    fn from(alias: MetricAlias) -> Self {
        alias.0
    }
}

impl std::fmt::Display for MetricAlias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Sparkplug data types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DataType {
    /// Unknown or unsupported type
    Unknown = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UNKNOWN,
    /// Signed 8-bit integer
    Int8 = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_INT8,
    /// Signed 16-bit integer
    Int16 = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_INT16,
    /// Signed 32-bit integer
    Int32 = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_INT32,
    /// Signed 64-bit integer
    Int64 = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_INT64,
    /// Unsigned 8-bit integer
    UInt8 = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UINT8,
    /// Unsigned 16-bit integer
    UInt16 = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UINT16,
    /// Unsigned 32-bit integer
    UInt32 = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UINT32,
    /// Unsigned 64-bit integer
    UInt64 = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UINT64,
    /// 32-bit floating point
    Float = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_FLOAT,
    /// 64-bit floating point
    Double = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_DOUBLE,
    /// Boolean value
    Boolean = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_BOOLEAN,
    /// String value
    String = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_STRING,
    /// DateTime value
    DateTime = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_DATETIME,
    /// Text value
    Text = sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_TEXT,
}

impl From<sys::sparkplug_data_type_t> for DataType {
    fn from(dt: sys::sparkplug_data_type_t) -> Self {
        match dt {
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_INT8 => DataType::Int8,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_INT16 => DataType::Int16,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_INT32 => DataType::Int32,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_INT64 => DataType::Int64,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UINT8 => DataType::UInt8,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UINT16 => DataType::UInt16,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UINT32 => DataType::UInt32,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_UINT64 => DataType::UInt64,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_FLOAT => DataType::Float,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_DOUBLE => DataType::Double,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_BOOLEAN => DataType::Boolean,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_STRING => DataType::String,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_DATETIME => DataType::DateTime,
            sys::sparkplug_data_type_t_SPARKPLUG_DATA_TYPE_TEXT => DataType::Text,
            _ => DataType::Unknown,
        }
    }
}

/// Metric value type.
#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    /// Signed 8-bit integer value
    Int8(i8),
    /// Signed 16-bit integer value
    Int16(i16),
    /// Signed 32-bit integer value
    Int32(i32),
    /// Signed 64-bit integer value
    Int64(i64),
    /// Unsigned 8-bit integer value
    UInt8(u8),
    /// Unsigned 16-bit integer value
    UInt16(u16),
    /// Unsigned 32-bit integer value
    UInt32(u32),
    /// Unsigned 64-bit integer value
    UInt64(u64),
    /// 32-bit floating point value
    Float(f32),
    /// 64-bit floating point value
    Double(f64),
    /// Boolean value
    Boolean(bool),
    /// String value
    String(String),
    /// Null value
    Null,
}

/// Metric information.
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric name (if present)
    pub name: Option<String>,
    /// Metric alias (if present)
    pub alias: Option<MetricAlias>,
    /// Metric timestamp in milliseconds since Unix epoch (if present)
    pub timestamp: Option<u64>,
    /// Data type
    pub datatype: DataType,
    /// Metric value (or Null)
    pub value: MetricValue,
}
