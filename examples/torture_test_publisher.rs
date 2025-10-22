//! Torture Test Publisher
//!
//! Stress tests Sparkplug edge node implementation with:
//! - Continuous data publishing with variable scan rates
//! - Rebirth and reboot command handling
//! - Device-level messages (Motor01, Sensor01)
//! - Connection recovery and statistics tracking
//!
//! Usage: cargo run --example torture_test_publisher [broker_url] [group_id] [edge_node_id]

use sparkplug_rs::{
    Message, MessageType, PayloadBuilder, Publisher, PublisherConfig, Subscriber, SubscriberConfig,
};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ctrlc::set_handler(move || {
        println!("\n[PUBLISHER] Caught signal, shutting down gracefully...");
        RUNNING.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    let args: Vec<String> = std::env::args().collect();
    let broker_url = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("tcp://localhost:1883");
    let group_id = args.get(2).map(|s| s.as_str()).unwrap_or("TortureTest");
    let edge_node_id = args.get(3).map(|s| s.as_str()).unwrap_or("Publisher01");

    println!("=== Sparkplug Torture Test Publisher ===");
    println!("Broker: {}", broker_url);
    println!("Group: {}", group_id);
    println!("Edge Node: {}\n", edge_node_id);

    let mut torture_publisher = TortureTestPublisher::new(broker_url, group_id, edge_node_id)?;

    torture_publisher.initialize()?;

    while RUNNING.load(Ordering::SeqCst) {
        torture_publisher.run();

        if torture_publisher.connection_lost && RUNNING.load(Ordering::SeqCst) {
            torture_publisher.reconnect()?;
        }
    }

    torture_publisher.disconnect()?;

    println!("\n[PUBLISHER] Shutdown complete");
    Ok(())
}

struct TortureTestPublisher {
    broker_url: String,
    group_id: String,
    edge_node_id: String,
    publisher: Option<Publisher>,
    subscriber: Option<Subscriber>,
    message_count: Arc<AtomicI64>,
    reconnect_count: i32,
    connection_lost: bool,
    do_rebirth: Arc<AtomicBool>,
    scan_rate_ms: Arc<AtomicI64>,
}

impl TortureTestPublisher {
    fn new(
        broker_url: &str,
        group_id: &str,
        edge_node_id: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            broker_url: broker_url.to_string(),
            group_id: group_id.to_string(),
            edge_node_id: edge_node_id.to_string(),
            publisher: None,
            subscriber: None,
            message_count: Arc::new(AtomicI64::new(0)),
            reconnect_count: 0,
            connection_lost: false,
            do_rebirth: Arc::new(AtomicBool::new(false)),
            scan_rate_ms: Arc::new(AtomicI64::new(1000)),
        })
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connect()
    }

    fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[PUBLISHER] Connecting to broker: {}", self.broker_url);
        let pub_config = PublisherConfig::new(
            &self.broker_url,
            "torture_test_publisher",
            &self.group_id,
            &self.edge_node_id,
        );
        let mut publisher = Publisher::new(pub_config)?;
        publisher.connect()?;
        let sub_config = SubscriberConfig::new(
            &self.broker_url,
            "torture_test_publisher_cmd_sub",
            &self.group_id,
        );

        let mut subscriber = Subscriber::new(sub_config, Box::new(|_msg: Message| {}))?;
        let edge_node_id_cmd = self.edge_node_id.clone();
        let do_rebirth_cmd = Arc::clone(&self.do_rebirth);
        let scan_rate_ms_cmd = Arc::clone(&self.scan_rate_ms);

        subscriber.set_command_callback(Box::new(move |msg: Message| {
            Self::handle_command(&msg, &edge_node_id_cmd, &do_rebirth_cmd, &scan_rate_ms_cmd);
        }))?;

        subscriber.connect()?;
        subscriber.subscribe_node(&self.edge_node_id)?;

        println!("[PUBLISHER] Connected successfully");

        if self.reconnect_count > 0 {
            println!("[PUBLISHER] This is reconnection #{}", self.reconnect_count);
        }

        self.publisher = Some(publisher);
        self.subscriber = Some(subscriber);

        self.publish_birth()
    }

    fn handle_command(
        msg: &Message,
        edge_node_id: &str,
        do_rebirth: &Arc<AtomicBool>,
        scan_rate_ms: &Arc<AtomicI64>,
    ) {
        if let Ok(topic) = msg.parse_topic() {
            if let Some(MessageType::NCmd) = topic.message_type() {
                if topic.edge_node_id() == Some(edge_node_id) {
                    println!("[PUBLISHER] Received command: {}", msg.topic);

                    if let Ok(payload) = msg.parse_payload() {
                        for metric in payload.metrics().flatten() {
                            if let Some(ref name) = metric.name {
                                println!("[PUBLISHER]   Command: {}", name);

                                match name.as_str() {
                                    "Node Control/Rebirth" => {
                                        if let sparkplug_rs::MetricValue::Boolean(true) =
                                            metric.value
                                        {
                                            println!("[PUBLISHER]   -> Rebirth requested");
                                            do_rebirth.store(true, Ordering::SeqCst);
                                        }
                                    }
                                    "Node Control/Scan Rate" => {
                                        if let sparkplug_rs::MetricValue::Int64(rate) = metric.value
                                        {
                                            println!(
                                                "[PUBLISHER]   -> Scan rate changed to {}ms",
                                                rate
                                            );
                                            scan_rate_ms.store(rate, Ordering::SeqCst);
                                        }
                                    }
                                    "Node Control/Reboot" => {
                                        if let sparkplug_rs::MetricValue::Boolean(true) =
                                            metric.value
                                        {
                                            println!("[PUBLISHER]   -> Reboot requested (simulating crash in 2s...)");
                                            thread::spawn(|| {
                                                thread::sleep(Duration::from_secs(2));
                                                println!("[PUBLISHER] CRASH SIMULATION (exit without graceful shutdown)");
                                                std::process::exit(0);
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn publish_birth(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let publisher = self.publisher.as_mut().unwrap();

        let mut birth = PayloadBuilder::new()?;

        birth
            .add_bd_seq(publisher.bd_seq())?
            .add_node_control_rebirth(false)?
            .add_node_control_reboot(false)?
            .add_node_control_scan_rate(self.scan_rate_ms.load(Ordering::SeqCst))?
            .add_string("Properties/Software", "Torture Test Publisher")?
            .add_string("Properties/Version", "1.0.0")?
            .add_int64("Properties/Reconnects", self.reconnect_count as i64)?
            .add_double_with_alias("Temperature", 1, 20.0)?
            .add_double_with_alias("Pressure", 2, 101.3)?
            .add_double_with_alias("Humidity", 3, 45.0)?
            .add_int64_with_alias("Uptime", 4, 0)?
            .add_int64_with_alias("MessageCount", 5, self.message_count.load(Ordering::SeqCst))?;

        let birth_bytes = birth.serialize()?;
        publisher.publish_birth(&birth_bytes)?;

        println!(
            "[PUBLISHER] Published NBIRTH (bdSeq={}, seq={})",
            publisher.bd_seq(),
            publisher.seq()
        );

        self.publish_device_births()
    }

    fn publish_device_births(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let publisher = self.publisher.as_mut().unwrap();
        let mut motor_birth = PayloadBuilder::new()?;
        motor_birth
            .add_double_with_alias("RPM", 1, 1500.0)?
            .add_bool_with_alias("Running", 2, true)?
            .add_double_with_alias("Temperature", 3, 65.0)?;

        let motor_birth_bytes = motor_birth.serialize()?;
        publisher.publish_device_birth("Motor01", &motor_birth_bytes)?;
        println!(
            "[PUBLISHER] Published DBIRTH for Motor01 (seq={})",
            publisher.seq()
        );
        let mut sensor_birth = PayloadBuilder::new()?;
        sensor_birth
            .add_double_with_alias("Level", 1, 75.5)?
            .add_double_with_alias("Flow", 2, 120.0)?;

        let sensor_birth_bytes = sensor_birth.serialize()?;
        publisher.publish_device_birth("Sensor01", &sensor_birth_bytes)?;
        println!(
            "[PUBLISHER] Published DBIRTH for Sensor01 (seq={})",
            publisher.seq()
        );

        Ok(())
    }

    fn run(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        let mut temperature = 20.0;
        let mut pressure = 101.3;
        let mut humidity = 45.0;
        let start_time = Instant::now();

        println!("[PUBLISHER] Starting data publishing loop...");
        println!("[PUBLISHER] Send SIGINT (Ctrl+C) for graceful shutdown");
        println!("[PUBLISHER] Send NCMD 'Node Control/Reboot' for ungraceful crash\n");

        while RUNNING.load(Ordering::SeqCst) {
            if self.do_rebirth.load(Ordering::SeqCst) {
                println!("\n[PUBLISHER] *** EXECUTING REBIRTH ***");
                if let Some(publisher) = &mut self.publisher {
                    match publisher.rebirth() {
                        Ok(_) => {
                            println!(
                                "[PUBLISHER] Rebirth complete (new bdSeq={})",
                                publisher.bd_seq()
                            );
                            if let Err(e) = self.publish_device_births() {
                                eprintln!(
                                    "[PUBLISHER] Failed to publish device births after rebirth: {}",
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("[PUBLISHER] Rebirth failed: {}", e);
                        }
                    }
                }
                self.do_rebirth.store(false, Ordering::SeqCst);
                println!();
            }
            temperature += rng.random::<f64>() - 0.5;
            pressure += (rng.random::<f64>() - 0.5) * 0.1;
            humidity += (rng.random::<f64>() - 0.5) * 2.0;
            let uptime = start_time.elapsed().as_secs() as i64;
            let mut data = PayloadBuilder::new().unwrap();
            data.add_double_by_alias(1, temperature)
                .add_double_by_alias(2, pressure)
                .add_double_by_alias(3, humidity)
                .add_int64_by_alias(4, uptime)
                .add_int64_by_alias(5, self.message_count.load(Ordering::SeqCst));

            if let Some(publisher) = &mut self.publisher {
                match data.serialize() {
                    Ok(data_bytes) => match publisher.publish_data(&data_bytes) {
                        Ok(_) => {
                            self.message_count.fetch_add(1, Ordering::SeqCst);

                            let count = self.message_count.load(Ordering::SeqCst);
                            if count % 10 == 0 {
                                println!(
                                    "[PUBLISHER] Messages: {}, Seq: {}, Temp: {:.1}C",
                                    count,
                                    publisher.seq(),
                                    temperature
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("[PUBLISHER] Failed to publish NDATA: {}", e);
                            self.connection_lost = true;
                            break;
                        }
                    },
                    Err(e) => {
                        eprintln!("[PUBLISHER] Failed to serialize data: {}", e);
                    }
                }
            }

            thread::sleep(Duration::from_millis(
                self.scan_rate_ms.load(Ordering::SeqCst) as u64,
            ));
        }
    }

    fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n[PUBLISHER] Disconnecting...");

        if let Some(mut subscriber) = self.subscriber.take() {
            if let Err(e) = subscriber.disconnect() {
                eprintln!("[PUBLISHER] Subscriber disconnect failed: {}", e);
            }
        }

        if let Some(mut publisher) = self.publisher.take() {
            if let Err(e) = publisher.publish_death() {
                eprintln!("[PUBLISHER] Failed to publish NDEATH: {}", e);
            } else {
                println!(
                    "[PUBLISHER] Published NDEATH (bdSeq={})",
                    publisher.bd_seq()
                );
            }

            if let Err(e) = publisher.disconnect() {
                eprintln!("[PUBLISHER] Publisher disconnect failed: {}", e);
            } else {
                println!("[PUBLISHER] Disconnected gracefully");
            }
        }

        self.print_statistics();
        Ok(())
    }

    fn reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.reconnect_count += 1;
        println!(
            "\n[PUBLISHER] *** RECONNECTION ATTEMPT #{} ***",
            self.reconnect_count
        );

        let _ = self.disconnect();

        thread::sleep(Duration::from_secs(2));

        match self.connect() {
            Ok(_) => {
                self.connection_lost = false;
                Ok(())
            }
            Err(e) => {
                eprintln!("[PUBLISHER] Reconnection failed, will retry...");
                Err(e)
            }
        }
    }

    fn print_statistics(&self) {
        println!("\n[PUBLISHER] Session Statistics:");
        println!(
            "  Total messages published: {}",
            self.message_count.load(Ordering::SeqCst)
        );
        println!("  Reconnection count: {}", self.reconnect_count);
        if let Some(publisher) = &self.publisher {
            println!("  Final sequence: {}", publisher.seq());
            println!("  Final bdSeq: {}", publisher.bd_seq());
        }
    }
}
