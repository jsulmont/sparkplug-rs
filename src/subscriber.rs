//! Sparkplug Subscriber for receiving messages.

use crate::error::{Error, Result};
use crate::payload::Payload;
use crate::sys;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr;
use std::sync::{Arc, Mutex};

/// Message received by a subscriber.
#[derive(Debug, Clone)]
pub struct Message {
    /// MQTT topic string.
    pub topic: String,
    /// Raw protobuf payload data.
    pub payload_data: Vec<u8>,
}

impl Message {
    /// Parses the payload into a structured Payload object.
    pub fn parse_payload(&self) -> Result<Payload> {
        Payload::parse(&self.payload_data)
    }
}

/// Callback function type for receiving messages.
pub type MessageCallback = Box<dyn Fn(Message) + Send + 'static>;

/// Callback function type for receiving command messages (NCMD/DCMD).
pub type CommandCallback = Box<dyn Fn(Message) + Send + 'static>;

/// Configuration for a Sparkplug Subscriber.
#[derive(Clone)]
pub struct SubscriberConfig {
    /// MQTT broker URL (e.g., "tcp://localhost:1883").
    pub broker_url: String,
    /// Unique MQTT client identifier.
    pub client_id: String,
    /// Sparkplug group ID to subscribe to.
    pub group_id: String,
}

impl SubscriberConfig {
    /// Creates a new subscriber configuration.
    pub fn new(
        broker_url: impl Into<String>,
        client_id: impl Into<String>,
        group_id: impl Into<String>,
    ) -> Self {
        Self {
            broker_url: broker_url.into(),
            client_id: client_id.into(),
            group_id: group_id.into(),
        }
    }
}

/// Internal state for subscriber callbacks.
struct SubscriberCallbacks {
    message_callback: Option<MessageCallback>,
    command_callback: Option<CommandCallback>,
}

/// A Sparkplug Subscriber for receiving messages.
///
/// The Subscriber connects to an MQTT broker and receives Sparkplug messages
/// via callbacks. It supports:
/// - Subscribing to all messages in a group
/// - Subscribing to specific edge nodes
/// - Subscribing to STATE messages
/// - Sequence validation and node state tracking
///
/// The underlying C++ implementation is thread-safe.
///
/// # Example
///
/// ```no_run
/// use sparkplug_rs::{Subscriber, SubscriberConfig, Message};
///
/// let config = SubscriberConfig::new(
///     "tcp://localhost:1883",
///     "my_subscriber",
///     "Energy"
/// );
///
/// let mut subscriber = Subscriber::new(config, Box::new(|msg: Message| {
///     println!("Received message on topic: {}", msg.topic);
///     if let Ok(payload) = msg.parse_payload() {
///         println!("  Metrics: {}", payload.metric_count());
///     }
/// }))?;
///
/// subscriber.connect()?;
/// subscriber.subscribe_all()?;
///
/// // Keep running to receive messages
/// std::thread::sleep(std::time::Duration::from_secs(60));
///
/// subscriber.disconnect()?;
/// # Ok::<(), sparkplug_rs::Error>(())
/// ```
pub struct Subscriber {
    inner: *mut sys::sparkplug_subscriber_t,
    callbacks: Arc<Mutex<SubscriberCallbacks>>,
}

impl Subscriber {
    /// Creates a new Subscriber with the given configuration and message callback.
    pub fn new(config: SubscriberConfig, message_callback: MessageCallback) -> Result<Self> {
        let callbacks = Arc::new(Mutex::new(SubscriberCallbacks {
            message_callback: Some(message_callback),
            command_callback: None,
        }));

        let broker_url = CString::new(config.broker_url)?;
        let client_id = CString::new(config.client_id)?;
        let group_id = CString::new(config.group_id)?;

        // Create a raw pointer to the callbacks Arc to pass as user_data
        let user_data = Arc::into_raw(Arc::clone(&callbacks)) as *mut c_void;

        let inner = unsafe {
            sys::sparkplug_subscriber_create(
                broker_url.as_ptr(),
                client_id.as_ptr(),
                group_id.as_ptr(),
                Some(Self::message_callback_wrapper),
                user_data,
            )
        };

        if inner.is_null() {
            // Clean up the Arc we created for user_data
            unsafe {
                Arc::from_raw(user_data as *const Mutex<SubscriberCallbacks>);
            }
            return Err(Error::CreateFailed {
                component: "Subscriber",
                details: "sparkplug_subscriber_create returned null".to_string(),
            });
        }

        Ok(Self { inner, callbacks })
    }

    /// Internal wrapper for the message callback.
    unsafe extern "C" fn message_callback_wrapper(
        topic: *const i8,
        payload_data: *const u8,
        payload_len: usize,
        user_data: *mut c_void,
    ) {
        if user_data.is_null() {
            return;
        }

        // Reconstruct the Arc (but don't drop it - just borrow)
        let callbacks = unsafe { &*(user_data as *const Mutex<SubscriberCallbacks>) };

        let topic_str = if topic.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(topic).to_string_lossy().into_owned() }
        };

        let payload_vec = if payload_data.is_null() || payload_len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(payload_data, payload_len).to_vec() }
        };

        let message = Message {
            topic: topic_str,
            payload_data: payload_vec,
        };

        if let Ok(guard) = callbacks.lock() {
            if let Some(ref callback) = guard.message_callback {
                callback(message);
            }
        }
    }

    /// Internal wrapper for the command callback.
    unsafe extern "C" fn command_callback_wrapper(
        topic: *const i8,
        payload_data: *const u8,
        payload_len: usize,
        user_data: *mut c_void,
    ) {
        if user_data.is_null() {
            return;
        }

        let callbacks = unsafe { &*(user_data as *const Mutex<SubscriberCallbacks>) };

        let topic_str = if topic.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(topic).to_string_lossy().into_owned() }
        };

        let payload_vec = if payload_data.is_null() || payload_len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(payload_data, payload_len).to_vec() }
        };

        let message = Message {
            topic: topic_str,
            payload_data: payload_vec,
        };

        if let Ok(guard) = callbacks.lock() {
            if let Some(ref callback) = guard.command_callback {
                callback(message);
            }
        }
    }

    /// Sets a callback for receiving command messages (NCMD/DCMD).
    ///
    /// This callback is invoked in addition to the general message callback.
    pub fn set_command_callback(&mut self, callback: CommandCallback) -> Result<()> {
        if let Ok(mut guard) = self.callbacks.lock() {
            guard.command_callback = Some(callback);
        }

        let user_data = Arc::as_ptr(&self.callbacks) as *mut c_void;
        unsafe {
            sys::sparkplug_subscriber_set_command_callback(
                self.inner,
                Some(Self::command_callback_wrapper),
                user_data,
            );
        }
        Ok(())
    }

    /// Removes the command callback.
    pub fn clear_command_callback(&mut self) {
        if let Ok(mut guard) = self.callbacks.lock() {
            guard.command_callback = None;
        }

        unsafe {
            sys::sparkplug_subscriber_set_command_callback(self.inner, None, ptr::null_mut());
        }
    }

    /// Connects to the MQTT broker.
    pub fn connect(&mut self) -> Result<()> {
        let ret = unsafe { sys::sparkplug_subscriber_connect(self.inner) };
        if ret != 0 {
            return Err(Error::ConnectionFailed(
                "Failed to connect to MQTT broker".to_string(),
            ));
        }
        Ok(())
    }

    /// Disconnects from the MQTT broker.
    pub fn disconnect(&mut self) -> Result<()> {
        let ret = unsafe { sys::sparkplug_subscriber_disconnect(self.inner) };
        if ret != 0 {
            return Err(Error::OperationFailed {
                operation: "disconnect",
            });
        }
        Ok(())
    }

    /// Subscribes to all Sparkplug messages in the configured group.
    ///
    /// This subscribes to the wildcard topic: `spBv1.0/{group_id}/#`
    pub fn subscribe_all(&mut self) -> Result<()> {
        let ret = unsafe { sys::sparkplug_subscriber_subscribe_all(self.inner) };
        if ret != 0 {
            return Err(Error::OperationFailed {
                operation: "subscribe_all",
            });
        }
        Ok(())
    }

    /// Subscribes to messages from a specific edge node.
    ///
    /// This subscribes to: `spBv1.0/{group_id}/+/{edge_node_id}/#`
    pub fn subscribe_node(&mut self, edge_node_id: &str) -> Result<()> {
        let c_edge_node_id = CString::new(edge_node_id)?;
        let ret = unsafe {
            sys::sparkplug_subscriber_subscribe_node(self.inner, c_edge_node_id.as_ptr())
        };
        if ret != 0 {
            return Err(Error::OperationFailed {
                operation: "subscribe_node",
            });
        }
        Ok(())
    }

    /// Subscribes to STATE messages from a primary application.
    ///
    /// This subscribes to: `STATE/{host_id}`
    pub fn subscribe_state(&mut self, host_id: &str) -> Result<()> {
        let c_host_id = CString::new(host_id)?;
        let ret =
            unsafe { sys::sparkplug_subscriber_subscribe_state(self.inner, c_host_id.as_ptr()) };
        if ret != 0 {
            return Err(Error::OperationFailed {
                operation: "subscribe_state",
            });
        }
        Ok(())
    }
}

impl Drop for Subscriber {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                sys::sparkplug_subscriber_destroy(self.inner);
            }
        }

        // Clean up the Arc we created for callbacks
        // We need to reconstruct and drop it
        let user_data = Arc::as_ptr(&self.callbacks) as *mut c_void;
        if !user_data.is_null() {
            unsafe {
                // This reconstructs the Arc and then drops it, decrementing the ref count
                Arc::from_raw(user_data as *const Mutex<SubscriberCallbacks>);
            }
        }
    }
}

// The underlying C++ Subscriber is thread-safe (protected by mutexes).
unsafe impl Send for Subscriber {}
unsafe impl Sync for Subscriber {}
