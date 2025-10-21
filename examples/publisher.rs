//! Sparkplug B Rust Publisher Example
//!
//! This example demonstrates the Rust API for publishing Sparkplug messages.
//! It mirrors the functionality of the C publisher example.

use sparkplug_rs::{PayloadBuilder, Publisher, PublisherConfig, Result};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    println!("Sparkplug B Rust Publisher Example");
    println!("===================================\n");

    // Create publisher configuration
    let config = PublisherConfig::new(
        "tcp://localhost:1883",
        "rust_publisher_example",
        "Energy",
        "Gateway01",
    );

    // Create publisher
    let mut publisher = Publisher::new(config)?;
    println!("[OK] Publisher created");

    // Connect to broker
    publisher.connect()?;
    println!("[OK] Connected to broker");
    println!("  Initial bdSeq: {}", publisher.bd_seq());

    // Create NBIRTH payload with metrics and aliases
    let mut birth = PayloadBuilder::new()?;
    birth
        .add_double_with_alias("Temperature", 1, 20.5)?
        .add_double_with_alias("Voltage", 2, 230.0)?
        .add_bool_with_alias("Active", 3, true)?
        .add_int64_with_alias("Uptime", 4, 0)?
        .add_string("Properties/Hardware", "x86_64")?
        .add_string("Properties/OS", "Linux")?;

    let birth_bytes = birth.serialize()?;
    publisher.publish_birth(&birth_bytes)?;

    println!("[OK] Published NBIRTH");
    println!("  Sequence: {}", publisher.seq());
    println!("  bdSeq: {}", publisher.bd_seq());

    // Publish NDATA messages using aliases (Report by Exception)
    println!("\nPublishing NDATA messages...");

    for i in 0..10 {
        let mut data = PayloadBuilder::new()?;

        // Only include changed values (Report by Exception)
        let temp = 20.5 + (i as f64 * 0.1);
        let uptime = i as i64;

        data.add_double_by_alias(1, temp) // Temperature
            .add_int64_by_alias(4, uptime); // Uptime
                                            // Voltage and Active unchanged - not included

        let data_bytes = data.serialize()?;
        publisher.publish_data(&data_bytes)?;

        if (i + 1) % 5 == 0 {
            println!(
                "[OK] Published {} NDATA messages (seq: {})",
                i + 1,
                publisher.seq()
            );
        }

        thread::sleep(Duration::from_secs(1));
    }

    // Test rebirth
    println!("\nTesting rebirth...");
    publisher.rebirth()?;
    println!("[OK] Rebirth complete");
    println!("  New bdSeq: {}", publisher.bd_seq());
    println!("  Sequence reset to: {}", publisher.seq());

    // Publish a few more NDATA after rebirth
    println!("\nPublishing post-rebirth NDATA...");
    for i in 0..3 {
        let mut data = PayloadBuilder::new()?;
        data.add_double_by_alias(1, 25.0 + i as f64);

        let data_bytes = data.serialize()?;
        publisher.publish_data(&data_bytes)?;

        thread::sleep(Duration::from_secs(1));
    }

    println!(
        "[OK] Published 3 post-rebirth messages (seq: {})",
        publisher.seq()
    );

    // Test device-level messages
    println!("\nTesting device-level messages...");

    let mut device_birth = PayloadBuilder::new()?;
    device_birth
        .add_double_with_alias("Sensor/Temp", 10, 22.5)?
        .add_bool_with_alias("Sensor/Online", 11, true)?;

    let device_birth_bytes = device_birth.serialize()?;
    publisher.publish_device_birth("Sensor01", &device_birth_bytes)?;
    println!("[OK] Published DBIRTH for Sensor01");

    let mut device_data = PayloadBuilder::new()?;
    device_data.add_double_by_alias(10, 23.0);

    let device_data_bytes = device_data.serialize()?;
    publisher.publish_device_data("Sensor01", &device_data_bytes)?;
    println!("[OK] Published DDATA for Sensor01");

    thread::sleep(Duration::from_secs(1));

    publisher.publish_device_death("Sensor01")?;
    println!("[OK] Published DDEATH for Sensor01");

    // Clean disconnect
    println!("\nDisconnecting...");
    publisher.disconnect()?;
    println!("[OK] Disconnected (NDEATH sent via MQTT Will)");

    println!("\nRust publisher example complete!");
    println!("Final statistics:");
    println!("  Total messages sent: 16 (1 NBIRTH + 10 NDATA + 1 REBIRTH + 3 NDATA + 1 DBIRTH + 1 DDATA + 1 DDEATH)");

    Ok(())
}
