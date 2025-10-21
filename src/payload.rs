//! Sparkplug payload building and parsing.

use crate::error::{Error, Result};
use crate::sys;
use crate::types::{DataType, Metric, MetricAlias, MetricValue};
use std::ffi::CStr;

/// Maximum payload size for serialization.
const MAX_PAYLOAD_SIZE: usize = 65536;

/// A Sparkplug payload builder for creating NBIRTH, NDATA, and other messages.
///
/// This provides a type-safe, RAII wrapper around the C API's payload builder.
///
/// # Example
///
/// ```no_run
/// use sparkplug_rs::PayloadBuilder;
///
/// let mut builder = PayloadBuilder::new()?;
/// builder
///     .add_double_with_alias("Temperature", 1, 20.5)?
///     .add_bool_with_alias("Active", 2, true)?;
///
/// let bytes = builder.serialize()?;
/// # Ok::<(), sparkplug_rs::Error>(())
/// ```
pub struct PayloadBuilder {
    inner: *mut sys::sparkplug_payload_t,
}

impl PayloadBuilder {
    /// Creates a new payload builder.
    pub fn new() -> Result<Self> {
        let inner = unsafe { sys::sparkplug_payload_create() };
        if inner.is_null() {
            return Err(Error::CreateFailed {
                component: "PayloadBuilder",
                details: "sparkplug_payload_create returned null".to_string(),
            });
        }
        Ok(Self { inner })
    }

    /// Sets the payload-level timestamp in milliseconds since Unix epoch.
    pub fn set_timestamp(&mut self, timestamp: u64) -> &mut Self {
        unsafe {
            sys::sparkplug_payload_set_timestamp(self.inner, timestamp);
        }
        self
    }

    /// Sets the sequence number manually (not recommended in normal operation).
    pub fn set_seq(&mut self, seq: u64) -> &mut Self {
        unsafe {
            sys::sparkplug_payload_set_seq(self.inner, seq);
        }
        self
    }

    // Note: set_timestamp and set_seq don't take string parameters, so they remain infallible

    // ===== Metric functions by name only =====

    /// Adds an int8 metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_int8(&mut self, name: &str, value: i8) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_int8(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds an int16 metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_int16(&mut self, name: &str, value: i16) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_int16(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds an int32 metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_int32(&mut self, name: &str, value: i32) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_int32(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds an int64 metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_int64(&mut self, name: &str, value: i64) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_int64(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds a uint8 metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_uint8(&mut self, name: &str, value: u8) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_uint8(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds a uint16 metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_uint16(&mut self, name: &str, value: u16) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_uint16(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds a uint32 metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_uint32(&mut self, name: &str, value: u32) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_uint32(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds a uint64 metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_uint64(&mut self, name: &str, value: u64) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_uint64(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds a float metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_float(&mut self, name: &str, value: f32) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_float(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds a double metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_double(&mut self, name: &str, value: f64) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_double(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds a boolean metric by name.
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_bool(&mut self, name: &str, value: bool) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        unsafe {
            sys::sparkplug_payload_add_bool(self.inner, c_name.as_ptr(), value);
        }
        Ok(self)
    }

    /// Adds a string metric by name.
    ///
    /// Returns an error if the name or value contains null bytes.
    pub fn add_string(&mut self, name: &str, value: &str) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        let c_value = std::ffi::CString::new(value)?;
        unsafe {
            sys::sparkplug_payload_add_string(self.inner, c_name.as_ptr(), c_value.as_ptr());
        }
        Ok(self)
    }

    // ===== Metric functions with alias (for NBIRTH) =====

    /// Adds an int32 metric with both name and alias (for NBIRTH).
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_int32_with_alias(&mut self, name: &str, alias: impl Into<MetricAlias>, value: i32) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_int32_with_alias(self.inner, c_name.as_ptr(), alias, value);
        }
        Ok(self)
    }

    /// Adds an int64 metric with both name and alias (for NBIRTH).
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_int64_with_alias(&mut self, name: &str, alias: impl Into<MetricAlias>, value: i64) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_int64_with_alias(self.inner, c_name.as_ptr(), alias, value);
        }
        Ok(self)
    }

    /// Adds a uint32 metric with both name and alias (for NBIRTH).
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_uint32_with_alias(&mut self, name: &str, alias: impl Into<MetricAlias>, value: u32) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_uint32_with_alias(self.inner, c_name.as_ptr(), alias, value);
        }
        Ok(self)
    }

    /// Adds a uint64 metric with both name and alias (for NBIRTH).
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_uint64_with_alias(&mut self, name: &str, alias: impl Into<MetricAlias>, value: u64) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_uint64_with_alias(self.inner, c_name.as_ptr(), alias, value);
        }
        Ok(self)
    }

    /// Adds a float metric with both name and alias (for NBIRTH).
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_float_with_alias(&mut self, name: &str, alias: impl Into<MetricAlias>, value: f32) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_float_with_alias(self.inner, c_name.as_ptr(), alias, value);
        }
        Ok(self)
    }

    /// Adds a double metric with both name and alias (for NBIRTH).
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_double_with_alias(&mut self, name: &str, alias: impl Into<MetricAlias>, value: f64) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_double_with_alias(self.inner, c_name.as_ptr(), alias, value);
        }
        Ok(self)
    }

    /// Adds a boolean metric with both name and alias (for NBIRTH).
    ///
    /// Returns an error if the name contains null bytes.
    pub fn add_bool_with_alias(&mut self, name: &str, alias: impl Into<MetricAlias>, value: bool) -> Result<&mut Self> {
        let c_name = std::ffi::CString::new(name)?;
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_bool_with_alias(self.inner, c_name.as_ptr(), alias, value);
        }
        Ok(self)
    }

    // ===== Metric functions by alias only (for NDATA) =====

    /// Adds an int32 metric by alias only (for NDATA).
    pub fn add_int32_by_alias(&mut self, alias: impl Into<MetricAlias>, value: i32) -> &mut Self {
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_int32_by_alias(self.inner, alias, value);
        }
        self
    }

    /// Adds an int64 metric by alias only (for NDATA).
    pub fn add_int64_by_alias(&mut self, alias: impl Into<MetricAlias>, value: i64) -> &mut Self {
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_int64_by_alias(self.inner, alias, value);
        }
        self
    }

    /// Adds a uint32 metric by alias only (for NDATA).
    pub fn add_uint32_by_alias(&mut self, alias: impl Into<MetricAlias>, value: u32) -> &mut Self {
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_uint32_by_alias(self.inner, alias, value);
        }
        self
    }

    /// Adds a uint64 metric by alias only (for NDATA).
    pub fn add_uint64_by_alias(&mut self, alias: impl Into<MetricAlias>, value: u64) -> &mut Self {
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_uint64_by_alias(self.inner, alias, value);
        }
        self
    }

    /// Adds a float metric by alias only (for NDATA).
    pub fn add_float_by_alias(&mut self, alias: impl Into<MetricAlias>, value: f32) -> &mut Self {
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_float_by_alias(self.inner, alias, value);
        }
        self
    }

    /// Adds a double metric by alias only (for NDATA).
    pub fn add_double_by_alias(&mut self, alias: impl Into<MetricAlias>, value: f64) -> &mut Self {
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_double_by_alias(self.inner, alias, value);
        }
        self
    }

    /// Adds a boolean metric by alias only (for NDATA).
    pub fn add_bool_by_alias(&mut self, alias: impl Into<MetricAlias>, value: bool) -> &mut Self {
        let alias: u64 = alias.into().into();
        unsafe {
            sys::sparkplug_payload_add_bool_by_alias(self.inner, alias, value);
        }
        self
    }

    /// Serializes the payload to binary protobuf format.
    ///
    /// Returns a vector of bytes that can be published via Publisher.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; MAX_PAYLOAD_SIZE];
        let size = unsafe {
            sys::sparkplug_payload_serialize(self.inner, buffer.as_mut_ptr(), buffer.len())
        };

        if size == 0 {
            return Err(Error::SerializeFailed {
                required: MAX_PAYLOAD_SIZE,
            });
        }

        buffer.truncate(size);
        Ok(buffer)
    }

    /// Returns the raw C pointer (for internal use).
    #[allow(dead_code)]
    pub(crate) fn as_ptr(&self) -> *const sys::sparkplug_payload_t {
        self.inner
    }
}

impl Drop for PayloadBuilder {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                sys::sparkplug_payload_destroy(self.inner);
            }
        }
    }
}

unsafe impl Send for PayloadBuilder {}
unsafe impl Sync for PayloadBuilder {}

/// A parsed Sparkplug payload.
///
/// This provides read access to a payload's contents, including metrics.
pub struct Payload {
    inner: *mut sys::sparkplug_payload_t,
}

impl Payload {
    /// Parses a Sparkplug payload from binary protobuf data.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let inner = unsafe { sys::sparkplug_payload_parse(data.as_ptr(), data.len()) };
        if inner.is_null() {
            return Err(Error::ParseFailed);
        }
        Ok(Self { inner })
    }

    /// Gets the payload-level timestamp, if present.
    pub fn timestamp(&self) -> Option<u64> {
        let mut ts: u64 = 0;
        unsafe {
            if sys::sparkplug_payload_get_timestamp(self.inner, &mut ts) {
                Some(ts)
            } else {
                None
            }
        }
    }

    /// Gets the payload-level sequence number, if present.
    pub fn seq(&self) -> Option<u64> {
        let mut seq: u64 = 0;
        unsafe {
            if sys::sparkplug_payload_get_seq(self.inner, &mut seq) {
                Some(seq)
            } else {
                None
            }
        }
    }

    /// Gets the payload UUID, if present.
    pub fn uuid(&self) -> Option<&str> {
        unsafe {
            let uuid_ptr = sys::sparkplug_payload_get_uuid(self.inner);
            if uuid_ptr.is_null() {
                None
            } else {
                CStr::from_ptr(uuid_ptr).to_str().ok()
            }
        }
    }

    /// Returns the number of metrics in the payload.
    pub fn metric_count(&self) -> usize {
        unsafe { sys::sparkplug_payload_get_metric_count(self.inner) }
    }

    /// Gets a metric at the specified index.
    pub fn metric_at(&self, index: usize) -> Result<Metric> {
        let count = self.metric_count();
        if index >= count {
            return Err(Error::InvalidMetricIndex { index, count });
        }

        let mut raw_metric: sys::sparkplug_metric_t = unsafe { std::mem::zeroed() };
        let success =
            unsafe { sys::sparkplug_payload_get_metric_at(self.inner, index, &mut raw_metric) };

        if !success {
            return Err(Error::InvalidMetricIndex { index, count });
        }

        let name = if raw_metric.has_name && !raw_metric.name.is_null() {
            unsafe { Some(CStr::from_ptr(raw_metric.name).to_str()?.to_string()) }
        } else {
            None
        };

        let alias = if raw_metric.has_alias {
            Some(MetricAlias::new(raw_metric.alias))
        } else {
            None
        };

        let timestamp = if raw_metric.has_timestamp {
            Some(raw_metric.timestamp)
        } else {
            None
        };

        let datatype = DataType::from(raw_metric.datatype);

        let value = if raw_metric.is_null {
            MetricValue::Null
        } else {
            // Access union fields using .as_ref() to get the inner value
            match datatype {
                DataType::Int8 => unsafe {
                    MetricValue::Int8(*raw_metric.value.int8_value.as_ref())
                },
                DataType::Int16 => unsafe {
                    MetricValue::Int16(*raw_metric.value.int16_value.as_ref())
                },
                DataType::Int32 => unsafe {
                    MetricValue::Int32(*raw_metric.value.int32_value.as_ref())
                },
                DataType::Int64 => unsafe {
                    MetricValue::Int64(*raw_metric.value.int64_value.as_ref())
                },
                DataType::UInt8 => unsafe {
                    MetricValue::UInt8(*raw_metric.value.uint8_value.as_ref())
                },
                DataType::UInt16 => unsafe {
                    MetricValue::UInt16(*raw_metric.value.uint16_value.as_ref())
                },
                DataType::UInt32 => unsafe {
                    MetricValue::UInt32(*raw_metric.value.uint32_value.as_ref())
                },
                DataType::UInt64 => unsafe {
                    MetricValue::UInt64(*raw_metric.value.uint64_value.as_ref())
                },
                DataType::Float => unsafe {
                    MetricValue::Float(*raw_metric.value.float_value.as_ref())
                },
                DataType::Double => unsafe {
                    MetricValue::Double(*raw_metric.value.double_value.as_ref())
                },
                DataType::Boolean => unsafe {
                    MetricValue::Boolean(*raw_metric.value.boolean_value.as_ref())
                },
                DataType::String | DataType::Text => unsafe {
                    let string_ptr = *raw_metric.value.string_value.as_ref();
                    if string_ptr.is_null() {
                        MetricValue::Null
                    } else {
                        MetricValue::String(CStr::from_ptr(string_ptr).to_str()?.to_string())
                    }
                },
                _ => MetricValue::Null,
            }
        };

        Ok(Metric {
            name,
            alias,
            timestamp,
            datatype,
            value,
        })
    }

    /// Returns an iterator over all metrics in the payload.
    pub fn metrics(&self) -> MetricIterator<'_> {
        MetricIterator {
            payload: self,
            index: 0,
            count: self.metric_count(),
        }
    }
}

impl Drop for Payload {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                sys::sparkplug_payload_destroy(self.inner);
            }
        }
    }
}

unsafe impl Send for Payload {}
unsafe impl Sync for Payload {}

/// Iterator over metrics in a payload.
pub struct MetricIterator<'a> {
    payload: &'a Payload,
    index: usize,
    count: usize,
}

impl<'a> Iterator for MetricIterator<'a> {
    type Item = Result<Metric>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            None
        } else {
            let result = self.payload.metric_at(self.index);
            self.index += 1;
            Some(result)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for MetricIterator<'a> {}
