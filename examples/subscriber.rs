//! Sparkplug B Rust Subscriber Example
//!
//! This example demonstrates the Rust API for subscribing to Sparkplug messages.
//! It mirrors the functionality of the C subscriber example.

use sparkplug_rs::{Message, Result, Subscriber, SubscriberConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<()> {
    println!("Sparkplug B Rust Subscriber Example");
    println!("====================================\n");

    // Setup signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Create subscriber configuration
    let config = SubscriberConfig::new("tcp://localhost:1883", "rust_subscriber_example", "Energy");

    // Create subscriber with message callback
    let mut subscriber = Subscriber::new(
        config,
        Box::new(|msg: Message| {
            println!("\n=== Message Received ===");
            println!("Topic: {}", msg.topic);

            // Parse topic to get structured information
            if let Ok(parsed) = msg.parse_topic() {
                if let Some(msg_type) = parsed.message_type() {
                    print!("Type: {} ", msg_type);
                    if msg_type.is_birth() {
                        print!("(Birth Certificate) ");
                    } else if msg_type.is_death() {
                        print!("(Death Certificate) ");
                    } else if msg_type.is_data() {
                        print!("(Data Update) ");
                    } else if msg_type.is_command() {
                        print!("(Command) ");
                    }
                    println!();

                    if let Some(group) = parsed.group_id() {
                        println!("Group: {}", group);
                    }
                    if let Some(node) = parsed.edge_node_id() {
                        println!("Edge Node: {}", node);
                    }
                    if let Some(device) = parsed.device_id() {
                        println!("Device: {}", device);
                    }
                } else if let Some(host) = parsed.host_id() {
                    println!("Type: STATE (SCADA Host)");
                    println!("Host: {}", host);
                }
            }

            println!("Payload size: {} bytes", msg.payload_data.len());

            // Parse the protobuf payload
            match msg.parse_payload() {
                Ok(payload) => {
                    // Print payload-level fields
                    if let Some(timestamp) = payload.timestamp() {
                        println!("Timestamp: {}", timestamp);
                    }

                    if let Some(seq) = payload.seq() {
                        println!("Sequence: {}", seq);
                    }

                    if let Some(uuid) = payload.uuid() {
                        println!("UUID: {}", uuid);
                    }

                    // Print all metrics
                    let metric_count = payload.metric_count();
                    println!("Metrics ({}):", metric_count);

                    for (i, metric_result) in payload.metrics().enumerate() {
                        match metric_result {
                            Ok(metric) => {
                                print!("  [{}] ", i);

                                // Print metric name or alias
                                if let Some(name) = &metric.name {
                                    print!("{}", name);
                                } else if let Some(alias) = metric.alias {
                                    print!("<alias {}>", alias);
                                } else {
                                    print!("<unnamed>");
                                }

                                // Print value
                                print!(" = ");
                                use sparkplug_rs::MetricValue;
                                match metric.value {
                                    MetricValue::Null => println!("NULL"),
                                    MetricValue::Int8(v) => println!("{} (int8)", v),
                                    MetricValue::Int16(v) => println!("{} (int16)", v),
                                    MetricValue::Int32(v) => println!("{} (int32)", v),
                                    MetricValue::Int64(v) => println!("{} (int64)", v),
                                    MetricValue::UInt8(v) => println!("{} (uint8)", v),
                                    MetricValue::UInt16(v) => println!("{} (uint16)", v),
                                    MetricValue::UInt32(v) => println!("{} (uint32)", v),
                                    MetricValue::UInt64(v) => println!("{} (uint64)", v),
                                    MetricValue::Float(v) => println!("{} (float)", v),
                                    MetricValue::Double(v) => println!("{} (double)", v),
                                    MetricValue::Boolean(v) => println!("{} (bool)", v),
                                    MetricValue::String(ref s) => println!("\"{}\" (string)", s),
                                }
                            }
                            Err(e) => {
                                eprintln!("  [{}] Error reading metric: {}", i, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse payload: {}", e);
                }
            }

            println!("========================");
        }),
    )?;

    println!("[OK] Subscriber created");

    // Optionally set a command callback
    subscriber.set_command_callback(Box::new(|msg: Message| {
        println!("\n>>> COMMAND Received <<<");
        println!("Topic: {}", msg.topic);
    }))?;

    // Connect to broker
    subscriber.connect()?;
    println!("[OK] Connected to broker");

    // Subscribe to all messages in the Energy group
    subscriber.subscribe_all()?;
    println!("[OK] Subscribed to spBv1.0/Energy/#");
    println!("\nListening for messages (Ctrl+C to stop)...");

    // Keep running and processing messages
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(100));
    }

    println!("\n\nShutting down...");

    // Disconnect
    subscriber.disconnect()?;
    println!("[OK] Disconnected from broker");

    println!("[OK] Subscriber destroyed");
    println!("\nRust subscriber example complete!");

    Ok(())
}
