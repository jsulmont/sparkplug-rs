//! Sparkplug Publisher for publishing node and device data.

use crate::error::{Error, Result};
use crate::sys;
use std::ffi::CString;

/// Configuration for a Sparkplug Publisher.
#[derive(Debug, Clone)]
pub struct PublisherConfig {
    /// MQTT broker URL (e.g., "tcp://localhost:1883").
    pub broker_url: String,
    /// Unique MQTT client identifier.
    pub client_id: String,
    /// Sparkplug group ID.
    pub group_id: String,
    /// Edge node identifier.
    pub edge_node_id: String,
}

impl PublisherConfig {
    /// Creates a new publisher configuration.
    pub fn new(
        broker_url: impl Into<String>,
        client_id: impl Into<String>,
        group_id: impl Into<String>,
        edge_node_id: impl Into<String>,
    ) -> Self {
        Self {
            broker_url: broker_url.into(),
            client_id: client_id.into(),
            group_id: group_id.into(),
            edge_node_id: edge_node_id.into(),
        }
    }
}

/// A Sparkplug Publisher for edge nodes.
///
/// The Publisher handles the complete lifecycle of a Sparkplug edge node:
/// - NBIRTH (Node Birth) on connection
/// - NDATA (Node Data) for updates
/// - NDEATH (Node Death) via MQTT Last Will Testament
/// - Sequence number management
/// - Birth/Death sequence (bdSeq) tracking
///
/// The underlying C++ implementation is thread-safe, so this type implements
/// Send + Sync.
///
/// # Example
///
/// ```no_run
/// use sparkplug_rs::{Publisher, PublisherConfig, PayloadBuilder};
///
/// let config = PublisherConfig::new(
///     "tcp://localhost:1883",
///     "my_publisher",
///     "Energy",
///     "Gateway01"
/// );
///
/// let mut publisher = Publisher::new(config)?;
/// publisher.connect()?;
///
/// // Create and publish NBIRTH
/// let mut birth = PayloadBuilder::new()?;
/// birth.add_double_with_alias("Temperature", 1, 20.5);
/// let birth_bytes = birth.serialize()?;
/// publisher.publish_birth(&birth_bytes)?;
///
/// // Publish NDATA updates
/// let mut data = PayloadBuilder::new()?;
/// data.add_double_by_alias(1, 21.0);
/// let data_bytes = data.serialize()?;
/// publisher.publish_data(&data_bytes)?;
///
/// publisher.disconnect()?;
/// # Ok::<(), sparkplug_rs::Error>(())
/// ```
pub struct Publisher {
    inner: *mut sys::sparkplug_publisher_t,
}

impl Publisher {
    /// Creates a new Publisher with the given configuration.
    pub fn new(config: PublisherConfig) -> Result<Self> {
        let broker_url = CString::new(config.broker_url)?;
        let client_id = CString::new(config.client_id)?;
        let group_id = CString::new(config.group_id)?;
        let edge_node_id = CString::new(config.edge_node_id)?;

        let inner = unsafe {
            sys::sparkplug_publisher_create(
                broker_url.as_ptr(),
                client_id.as_ptr(),
                group_id.as_ptr(),
                edge_node_id.as_ptr(),
            )
        };

        if inner.is_null() {
            return Err(Error::CreateFailed {
                component: "Publisher",
                details: "sparkplug_publisher_create returned null".to_string(),
            });
        }

        Ok(Self { inner })
    }

    /// Connects to the MQTT broker.
    ///
    /// This sets up the NDEATH message as the MQTT Last Will Testament before connecting.
    pub fn connect(&mut self) -> Result<()> {
        let ret = unsafe { sys::sparkplug_publisher_connect(self.inner) };
        if ret != 0 {
            return Err(Error::ConnectionFailed(
                "Failed to connect to MQTT broker".to_string(),
            ));
        }
        Ok(())
    }

    /// Disconnects from the MQTT broker.
    ///
    /// The NDEATH message is sent automatically via MQTT Last Will Testament.
    pub fn disconnect(&mut self) -> Result<()> {
        let ret = unsafe { sys::sparkplug_publisher_disconnect(self.inner) };
        if ret != 0 {
            return Err(Error::OperationFailed {
                operation: "disconnect",
            });
        }
        Ok(())
    }

    /// Publishes an NBIRTH (Node Birth) message.
    ///
    /// This must be called after connect() and before any publish_data() calls.
    /// The payload should contain all metrics with both names and aliases.
    pub fn publish_birth(&mut self, payload: &[u8]) -> Result<()> {
        let ret = unsafe {
            sys::sparkplug_publisher_publish_birth(self.inner, payload.as_ptr(), payload.len())
        };
        if ret != 0 {
            return Err(Error::PublishFailed {
                message_type: "NBIRTH",
                details: "publish_birth failed".to_string(),
            });
        }
        Ok(())
    }

    /// Publishes an NDATA (Node Data) message.
    ///
    /// The sequence number is automatically incremented.
    /// The payload should typically use aliases only for bandwidth efficiency.
    pub fn publish_data(&mut self, payload: &[u8]) -> Result<()> {
        let ret = unsafe {
            sys::sparkplug_publisher_publish_data(self.inner, payload.as_ptr(), payload.len())
        };
        if ret != 0 {
            return Err(Error::PublishFailed {
                message_type: "NDATA",
                details: "publish_data failed".to_string(),
            });
        }
        Ok(())
    }

    /// Publishes an NDEATH (Node Death) message.
    ///
    /// Normally not needed as NDEATH is sent automatically on disconnect.
    pub fn publish_death(&mut self) -> Result<()> {
        let ret = unsafe { sys::sparkplug_publisher_publish_death(self.inner) };
        if ret != 0 {
            return Err(Error::PublishFailed {
                message_type: "NDEATH",
                details: "publish_death failed".to_string(),
            });
        }
        Ok(())
    }

    /// Triggers a rebirth (publishes new NBIRTH with incremented bdSeq).
    ///
    /// This is typically called in response to an NCMD rebirth command.
    pub fn rebirth(&mut self) -> Result<()> {
        let ret = unsafe { sys::sparkplug_publisher_rebirth(self.inner) };
        if ret != 0 {
            return Err(Error::OperationFailed {
                operation: "rebirth",
            });
        }
        Ok(())
    }

    /// Gets the current message sequence number (0-255).
    pub fn seq(&self) -> u64 {
        unsafe { sys::sparkplug_publisher_get_seq(self.inner) }
    }

    /// Gets the current birth/death sequence number.
    pub fn bd_seq(&self) -> u64 {
        unsafe { sys::sparkplug_publisher_get_bd_seq(self.inner) }
    }

    /// Publishes a DBIRTH (Device Birth) message for a device.
    ///
    /// Must call publish_birth() before publishing any device births.
    pub fn publish_device_birth(&mut self, device_id: &str, payload: &[u8]) -> Result<()> {
        let c_device_id = CString::new(device_id)?;
        let ret = unsafe {
            sys::sparkplug_publisher_publish_device_birth(
                self.inner,
                c_device_id.as_ptr(),
                payload.as_ptr(),
                payload.len(),
            )
        };
        if ret != 0 {
            return Err(Error::PublishFailed {
                message_type: "DBIRTH",
                details: format!("publish_device_birth failed for device '{}'", device_id),
            });
        }
        Ok(())
    }

    /// Publishes a DDATA (Device Data) message for a device.
    ///
    /// Must call publish_device_birth() before the first publish_device_data().
    pub fn publish_device_data(&mut self, device_id: &str, payload: &[u8]) -> Result<()> {
        let c_device_id = CString::new(device_id)?;
        let ret = unsafe {
            sys::sparkplug_publisher_publish_device_data(
                self.inner,
                c_device_id.as_ptr(),
                payload.as_ptr(),
                payload.len(),
            )
        };
        if ret != 0 {
            return Err(Error::PublishFailed {
                message_type: "DDATA",
                details: format!("publish_device_data failed for device '{}'", device_id),
            });
        }
        Ok(())
    }

    /// Publishes a DDEATH (Device Death) message for a device.
    pub fn publish_device_death(&mut self, device_id: &str) -> Result<()> {
        let c_device_id = CString::new(device_id)?;
        let ret = unsafe {
            sys::sparkplug_publisher_publish_device_death(self.inner, c_device_id.as_ptr())
        };
        if ret != 0 {
            return Err(Error::PublishFailed {
                message_type: "DDEATH",
                details: format!("publish_device_death failed for device '{}'", device_id),
            });
        }
        Ok(())
    }

    /// Publishes an NCMD (Node Command) message to another edge node.
    pub fn publish_node_command(
        &mut self,
        target_edge_node_id: &str,
        payload: &[u8],
    ) -> Result<()> {
        let c_target = CString::new(target_edge_node_id)?;
        let ret = unsafe {
            sys::sparkplug_publisher_publish_node_command(
                self.inner,
                c_target.as_ptr(),
                payload.as_ptr(),
                payload.len(),
            )
        };
        if ret != 0 {
            return Err(Error::PublishFailed {
                message_type: "NCMD",
                details: format!(
                    "publish_node_command failed for node '{}'",
                    target_edge_node_id
                ),
            });
        }
        Ok(())
    }

    /// Publishes a DCMD (Device Command) message to a device on another edge node.
    pub fn publish_device_command(
        &mut self,
        target_edge_node_id: &str,
        target_device_id: &str,
        payload: &[u8],
    ) -> Result<()> {
        let c_edge_node = CString::new(target_edge_node_id)?;
        let c_device = CString::new(target_device_id)?;
        let ret = unsafe {
            sys::sparkplug_publisher_publish_device_command(
                self.inner,
                c_edge_node.as_ptr(),
                c_device.as_ptr(),
                payload.as_ptr(),
                payload.len(),
            )
        };
        if ret != 0 {
            return Err(Error::PublishFailed {
                message_type: "DCMD",
                details: format!(
                    "publish_device_command failed for device '{}' on node '{}'",
                    target_device_id, target_edge_node_id
                ),
            });
        }
        Ok(())
    }
}

impl Drop for Publisher {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                sys::sparkplug_publisher_destroy(self.inner);
            }
        }
    }
}

// The underlying C++ Publisher is thread-safe (protected by mutexes).
unsafe impl Send for Publisher {}
unsafe impl Sync for Publisher {}
