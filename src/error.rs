//! Error types for the Sparkplug Rust API.

use thiserror::Error;

/// Result type alias for Sparkplug operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur when using the Sparkplug API.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to create a Sparkplug component (publisher, subscriber, or payload).
    #[error("Failed to create {component}: {details}")]
    CreateFailed {
        /// The component that failed to be created
        component: &'static str,
        /// Additional details about the failure
        details: String,
    },

    /// An operation failed at the C API level.
    #[error("Operation failed: {operation}")]
    OperationFailed {
        /// The operation that failed
        operation: &'static str,
    },

    /// Failed to connect to MQTT broker.
    #[error("Failed to connect to broker: {0}")]
    ConnectionFailed(String),

    /// Failed to publish a message.
    #[error("Failed to publish {message_type}: {details}")]
    PublishFailed {
        /// The type of message that failed to publish
        message_type: &'static str,
        /// Additional details about the failure
        details: String,
    },

    /// Failed to serialize a payload.
    #[error("Failed to serialize payload: buffer too small (need at least {required} bytes)")]
    SerializeFailed {
        /// The required buffer size in bytes
        required: usize,
    },

    /// Failed to parse a payload.
    #[error("Failed to parse payload: invalid protobuf data")]
    ParseFailed,

    /// Invalid metric index.
    #[error("Invalid metric index: {index} (payload has {count} metrics)")]
    InvalidMetricIndex {
        /// The invalid index that was requested
        index: usize,
        /// The actual metric count in the payload
        count: usize,
    },

    /// A null pointer was encountered unexpectedly.
    #[error("Unexpected null pointer in {context}")]
    NullPointer {
        /// Context where the null pointer was encountered
        context: &'static str,
    },

    /// UTF-8 conversion error.
    #[error("Invalid UTF-8 string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    /// String contains null byte.
    #[error("String contains null byte: {0}")]
    NulError(#[from] std::ffi::NulError),
}
