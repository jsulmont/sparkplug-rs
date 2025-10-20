//! Idiomatic Rust bindings for Sparkplug B 2.2 protocol.
//!
//! This library provides safe, ergonomic Rust wrappers around the C API of the
//! Sparkplug B C++ library. It enables Industrial IoT applications to publish
//! and subscribe to Sparkplug messages over MQTT.
//!
//! # Features
//!
//! - **Thread-safe**: All types implement `Send` + `Sync` (underlying C++ is thread-safe)
//! - **RAII semantics**: Automatic resource cleanup via `Drop`
//! - **Type-safe**: Idiomatic Rust types and error handling
//! - **Zero-copy where possible**: Efficient FFI bindings
//! - **Iterator support**: Iterate over metrics in payloads
//!
//! # Architecture
//!
//! The library is organized into several modules:
//!
//! - [`Publisher`]: Publish node and device data (NBIRTH, NDATA, DBIRTH, DDATA)
//! - [`Subscriber`]: Subscribe to messages with callback handlers
//! - [`PayloadBuilder`]: Build payloads with type-safe metric additions
//! - [`Payload`]: Parse and read received payloads
//!
//! # Example: Publisher
//!
//! ```no_run
//! use sparkplug_rs::{Publisher, PublisherConfig, PayloadBuilder};
//!
//! # fn main() -> Result<(), sparkplug_rs::Error> {
//! let config = PublisherConfig::new(
//!     "tcp://localhost:1883",
//!     "my_publisher",
//!     "Energy",
//!     "Gateway01"
//! );
//!
//! let mut publisher = Publisher::new(config)?;
//! publisher.connect()?;
//!
//! // Create NBIRTH with metrics and aliases
//! let mut birth = PayloadBuilder::new()?;
//! birth
//!     .add_double_with_alias("Temperature", 1, 20.5)
//!     .add_bool_with_alias("Active", 2, true);
//!
//! let birth_bytes = birth.serialize()?;
//! publisher.publish_birth(&birth_bytes)?;
//!
//! // Publish NDATA updates using aliases
//! let mut data = PayloadBuilder::new()?;
//! data.add_double_by_alias(1, 21.0);
//!
//! let data_bytes = data.serialize()?;
//! publisher.publish_data(&data_bytes)?;
//!
//! publisher.disconnect()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Subscriber
//!
//! ```no_run
//! use sparkplug_rs::{Subscriber, SubscriberConfig, Message};
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), sparkplug_rs::Error> {
//! let config = SubscriberConfig::new(
//!     "tcp://localhost:1883",
//!     "my_subscriber",
//!     "Energy"
//! );
//!
//! let mut subscriber = Subscriber::new(config, Box::new(|msg: Message| {
//!     println!("Topic: {}", msg.topic);
//!     if let Ok(payload) = msg.parse_payload() {
//!         println!("Metrics: {}", payload.metric_count());
//!         for metric_result in payload.metrics() {
//!             if let Ok(metric) = metric_result {
//!                 println!("  {:?}", metric);
//!             }
//!         }
//!     }
//! }))?;
//!
//! subscriber.connect()?;
//! subscriber.subscribe_all()?;
//!
//! // Keep running to receive messages
//! std::thread::sleep(Duration::from_secs(60));
//!
//! subscriber.disconnect()?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![allow(unsafe_op_in_unsafe_fn)]

mod sys;

pub mod error;
pub mod payload;
pub mod publisher;
pub mod subscriber;
pub mod types;

pub use error::{Error, Result};
pub use payload::{Payload, PayloadBuilder};
pub use publisher::{Publisher, PublisherConfig};
pub use subscriber::{Message, Subscriber, SubscriberConfig};
pub use types::{DataType, Metric, MetricValue};
