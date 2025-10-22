//! Torture Test Subscriber
//!
//! Stress tests Sparkplug PRIMARY application with:
//! - Monitoring all messages in a group
//! - Node state tracking (AWAKE/SLEEPING/WAKE_PENDING/UNKNOWN)
//! - Automatic rebirth requests with exponential backoff
//! - Sequence validation and error detection
//! - Optional command sending (rebirth/reboot)
//! - Optional connection cycling for resilience testing
//!
//! Usage: cargo run --example torture_test_subscriber -- [options]
//!
//! Options:
//!   --broker <url>    MQTT broker URL (default: tcp://localhost:1883)
//!   --group <id>      Sparkplug group ID (default: TortureTest)
//!   --id <id>         Subscriber identifier (default: 01)
//!   --commands        Enable sending commands to publishers
//!   --cycle <sec>     Cycle connection every N seconds (0=never)
//!   --help            Show help message

use sparkplug_rs::{
    Message, MessageType, PayloadBuilder, Publisher, PublisherConfig, Subscriber, SubscriberConfig,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

static RUNNING: AtomicBool = AtomicBool::new(true);

#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeSleepState {
    Unknown,
    Awake,
    Sleeping,
    WakePending,
}

impl NodeSleepState {
    fn code(&self) -> &'static str {
        match self {
            NodeSleepState::Unknown => "UNK",
            NodeSleepState::Awake => "AWK",
            NodeSleepState::Sleeping => "SLP",
            NodeSleepState::WakePending => "WKP",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            NodeSleepState::Unknown => "UNKNOWN",
            NodeSleepState::Awake => "AWAKE (online)",
            NodeSleepState::Sleeping => "SLEEPING (waiting for NBIRTH)",
            NodeSleepState::WakePending => "WAKE_PENDING (rebirth requested)",
        }
    }
}

#[derive(Debug, Clone)]
struct NodeStats {
    birth_count: i64,
    death_count: i64,
    data_count: i64,
    current_bd_seq: u64,
    last_seq: u8,
    state: NodeSleepState,
    last_death_time: Option<Instant>,
    last_wake_attempt: Option<Instant>,
    wake_attempt_count: i32,
}

impl NodeStats {
    fn new() -> Self {
        Self {
            birth_count: 0,
            death_count: 0,
            data_count: 0,
            current_bd_seq: 0,
            last_seq: 0,
            state: NodeSleepState::Unknown,
            last_death_time: None,
            last_wake_attempt: None,
            wake_attempt_count: 0,
        }
    }

    fn online(&self) -> bool {
        self.state == NodeSleepState::Awake
    }

    fn sleeping(&self) -> bool {
        self.state == NodeSleepState::Sleeping
    }

    fn wake_pending(&self) -> bool {
        self.state == NodeSleepState::WakePending
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ctrlc::set_handler(move || {
        println!("\n[SUBSCRIBER] Caught signal, shutting down...");
        RUNNING.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    let args: Vec<String> = std::env::args().collect();
    let mut broker_url = "tcp://localhost:1883".to_string();
    let mut group_id = "TortureTest".to_string();
    let mut subscriber_id = "01".to_string();
    let mut send_commands = false;
    let mut cycle_interval = 0;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--broker" => {
                if i + 1 < args.len() {
                    broker_url = args[i + 1].clone();
                    i += 1;
                }
            }
            "--group" => {
                if i + 1 < args.len() {
                    group_id = args[i + 1].clone();
                    i += 1;
                }
            }
            "--id" => {
                if i + 1 < args.len() {
                    subscriber_id = args[i + 1].clone();
                    i += 1;
                }
            }
            "--commands" => {
                send_commands = true;
            }
            "--cycle" => {
                if i + 1 < args.len() {
                    cycle_interval = args[i + 1].parse().unwrap_or(0);
                    i += 1;
                }
            }
            "--help" => {
                print_help(&args[0]);
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    println!("=== Sparkplug Torture Test Subscriber ===");
    println!("Broker: {}", broker_url);
    println!("Group: {}", group_id);
    println!("Subscriber ID: {}", subscriber_id);
    println!(
        "Commands: {}",
        if send_commands { "ENABLED" } else { "DISABLED" }
    );
    println!(
        "Connection cycling: {}\n",
        if cycle_interval > 0 {
            format!("{}s", cycle_interval)
        } else {
            "DISABLED".to_string()
        }
    );

    let mut torture_subscriber = TortureTestSubscriber::new(
        &broker_url,
        &group_id,
        &subscriber_id,
        send_commands,
        cycle_interval,
    )?;

    torture_subscriber.initialize()?;
    torture_subscriber.run();
    torture_subscriber.disconnect()?;
    torture_subscriber.print_statistics();

    println!("\n[SUBSCRIBER] Shutdown complete");
    Ok(())
}

fn print_help(program: &str) {
    println!("Usage: {} [options]", program);
    println!("Options:");
    println!("  --broker <url>    MQTT broker URL (default: tcp://localhost:1883)");
    println!("  --group <id>      Sparkplug group ID (default: TortureTest)");
    println!("  --id <id>         Subscriber identifier (default: 01)");
    println!("  --commands        Enable sending commands to publishers");
    println!("  --cycle <sec>     Cycle connection every N seconds (0=never, default: 0)");
    println!("  --help            Show this help");
}

struct TortureTestSubscriber {
    broker_url: String,
    group_id: String,
    subscriber_id: String,
    send_commands: bool,
    cycle_interval_sec: i32,
    subscriber: Option<Subscriber>,
    command_publisher: Option<Publisher>,
    node_stats: Arc<Mutex<HashMap<String, NodeStats>>>,
    messages_received: Arc<AtomicI64>,
    sequence_errors: Arc<AtomicI64>,
    reconnect_count: Arc<AtomicI64>,
}

impl TortureTestSubscriber {
    fn new(
        broker_url: &str,
        group_id: &str,
        subscriber_id: &str,
        send_commands: bool,
        cycle_interval_sec: i32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            broker_url: broker_url.to_string(),
            group_id: group_id.to_string(),
            subscriber_id: subscriber_id.to_string(),
            send_commands,
            cycle_interval_sec,
            subscriber: None,
            command_publisher: None,
            node_stats: Arc::new(Mutex::new(HashMap::new())),
            messages_received: Arc::new(AtomicI64::new(0)),
            sequence_errors: Arc::new(AtomicI64::new(0)),
            reconnect_count: Arc::new(AtomicI64::new(0)),
        })
    }

    fn log_prefix(&self, node_id: Option<&str>) -> String {
        let mut prefix = format!("[SUB{}]", self.subscriber_id);
        if let Some(node_id) = node_id {
            if let Ok(stats) = self.node_stats.lock() {
                if let Some(node_stats) = stats.get(node_id) {
                    prefix.push_str(&format!(" [{}]", node_stats.state.code()));
                }
            }
        }
        prefix
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connect()
    }

    fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{} Connecting to broker: {}",
            self.log_prefix(None),
            self.broker_url
        );
        let pub_config = PublisherConfig::new(
            &self.broker_url,
            format!("torture_test_cmd_{}", self.subscriber_id),
            &self.group_id,
            format!("CommandHost_{}", self.subscriber_id),
        );

        let mut command_publisher = Publisher::new(pub_config)?;
        command_publisher.connect()?;

        let mut birth = PayloadBuilder::new()?;
        birth.add_string("Host Type", "Torture Test Command Host")?;
        let birth_bytes = birth.serialize()?;
        command_publisher.publish_birth(&birth_bytes)?;

        println!("{} Command publisher ready", self.log_prefix(None));
        let sub_config = SubscriberConfig::new(
            &self.broker_url,
            format!("torture_test_sub_{}", self.subscriber_id),
            &self.group_id,
        );

        let node_stats = Arc::clone(&self.node_stats);
        let messages_received = Arc::clone(&self.messages_received);
        let sequence_errors = Arc::clone(&self.sequence_errors);
        let subscriber_id = self.subscriber_id.clone();

        let subscriber = Subscriber::new(
            sub_config,
            Box::new(move |msg: Message| {
                Self::handle_message_static(
                    &msg,
                    &node_stats,
                    &messages_received,
                    &sequence_errors,
                    &subscriber_id,
                );
            }),
        )?;

        let mut sub = subscriber;
        sub.connect()?;
        sub.subscribe_all()?;

        println!(
            "{} Connected and subscribed to group: {}",
            self.log_prefix(None),
            self.group_id
        );

        let reconnect_count = self.reconnect_count.load(Ordering::SeqCst);
        if reconnect_count > 0 {
            println!(
                "{} This is reconnection #{}",
                self.log_prefix(None),
                reconnect_count
            );
        }

        self.command_publisher = Some(command_publisher);
        self.subscriber = Some(sub);

        self.request_rebirth_for_known_nodes();

        Ok(())
    }

    fn handle_message_static(
        msg: &Message,
        node_stats: &Arc<Mutex<HashMap<String, NodeStats>>>,
        messages_received: &Arc<AtomicI64>,
        sequence_errors: &Arc<AtomicI64>,
        subscriber_id: &str,
    ) {
        messages_received.fetch_add(1, Ordering::SeqCst);

        if let Ok(topic) = msg.parse_topic() {
            if let (Some(msg_type), Some(edge_node_id)) =
                (topic.message_type(), topic.edge_node_id())
            {
                let log_prefix_fn = |state: Option<NodeSleepState>| {
                    if let Some(state) = state {
                        format!("[SUB{}] [{}]", subscriber_id, state.code())
                    } else {
                        format!("[SUB{}]", subscriber_id)
                    }
                };

                let mut stats_map = node_stats.lock().unwrap();
                let stats = stats_map
                    .entry(edge_node_id.to_string())
                    .or_insert_with(NodeStats::new);

                match msg_type {
                    MessageType::NBirth => {
                        stats.birth_count += 1;

                        let mut bd_seq = 0u64;
                        if let Ok(payload) = msg.parse_payload() {
                            if let Some(seq) = payload.seq() {
                                stats.last_seq = (seq & 0xFF) as u8;
                            }

                            for metric in payload.metrics().flatten() {
                                if metric.name.as_deref() == Some("bdSeq")
                                    || metric.name.as_deref() == Some("Node Control/bdSeq")
                                {
                                    if let sparkplug_rs::MetricValue::UInt64(seq) = metric.value {
                                        bd_seq = seq;
                                        break;
                                    } else if let sparkplug_rs::MetricValue::Int64(seq) =
                                        metric.value
                                    {
                                        bd_seq = seq as u64;
                                        break;
                                    }
                                }
                            }

                            stats.current_bd_seq = bd_seq;

                            let prev_state = stats.state;
                            stats.state = NodeSleepState::Awake;
                            stats.wake_attempt_count = 0;

                            print!(
                                "{} NBIRTH from {} (bdSeq={}, seq={}, metrics={}",
                                log_prefix_fn(Some(stats.state)),
                                edge_node_id,
                                bd_seq,
                                payload.seq().unwrap_or(0),
                                payload.metric_count()
                            );
                            if prev_state == NodeSleepState::Sleeping {
                                print!(", WOKE UP from sleep");
                            } else if prev_state == NodeSleepState::WakePending {
                                print!(", wake successful");
                            }
                            println!(")");
                        }
                    }

                    MessageType::NDeath => {
                        stats.death_count += 1;

                        let mut bd_seq = 0u64;
                        if let Ok(payload) = msg.parse_payload() {
                            bd_seq = payload.seq().unwrap_or(0);
                        }

                        stats.state = NodeSleepState::Sleeping;
                        stats.last_death_time = Some(Instant::now());
                        stats.wake_attempt_count = 0;

                        println!(
                            "{} NDEATH from {} (bdSeq={}) - entering SLEEP mode",
                            log_prefix_fn(Some(stats.state)),
                            edge_node_id,
                            bd_seq
                        );
                    }

                    MessageType::NData => {
                        if let Ok(payload) = msg.parse_payload() {
                            let seq = payload.seq().unwrap_or(0);
                            let seq_u8 = (seq & 0xFF) as u8;

                            if stats.sleeping() || stats.state == NodeSleepState::Unknown {
                                println!(
                                    "{} NDATA from {} (seq={}) - node alive! Requesting rebirth",
                                    log_prefix_fn(Some(stats.state)),
                                    edge_node_id,
                                    seq
                                );
                                stats.state = NodeSleepState::WakePending;
                                stats.last_wake_attempt = Some(Instant::now());
                                stats.wake_attempt_count += 1;
                                return;
                            }

                            if stats.wake_pending() {
                                println!(
                                    "{} NDATA from {} (seq={}) - waiting for NBIRTH, ignoring",
                                    log_prefix_fn(Some(stats.state)),
                                    edge_node_id,
                                    seq
                                );
                                return;
                            }

                            if !stats.online() {
                                eprintln!(
                                    "{} NDATA from {} in unexpected state, requesting rebirth",
                                    log_prefix_fn(Some(stats.state)),
                                    edge_node_id
                                );
                                stats.state = NodeSleepState::WakePending;
                                stats.last_wake_attempt = Some(Instant::now());
                                stats.wake_attempt_count += 1;
                                return;
                            }

                            stats.data_count += 1;

                            let expected_seq = ((stats.last_seq as u16 + 1) % 256) as u8;
                            if seq_u8 != expected_seq && stats.last_seq != 255 {
                                eprintln!(
                                    "{} SEQUENCE ERROR on {}: expected {}, got {}",
                                    log_prefix_fn(Some(stats.state)),
                                    edge_node_id,
                                    expected_seq,
                                    seq_u8
                                );
                                sequence_errors.fetch_add(1, Ordering::SeqCst);
                            }

                            stats.last_seq = seq_u8;

                            println!(
                                "{} NDATA from {} (seq={}, metrics={}, count={})",
                                log_prefix_fn(Some(stats.state)),
                                edge_node_id,
                                seq,
                                payload.metric_count(),
                                stats.data_count
                            );
                        }
                    }

                    MessageType::DBirth => {
                        if let Ok(payload) = msg.parse_payload() {
                            println!(
                                "{} DBIRTH from {}/{} (seq={}, metrics={})",
                                log_prefix_fn(None),
                                edge_node_id,
                                topic.device_id().unwrap_or("?"),
                                payload.seq().unwrap_or(0),
                                payload.metric_count()
                            );
                        }
                    }

                    MessageType::DData => {
                        if let Ok(payload) = msg.parse_payload() {
                            println!(
                                "{} DDATA from {}/{} (seq={}, metrics={})",
                                log_prefix_fn(None),
                                edge_node_id,
                                topic.device_id().unwrap_or("?"),
                                payload.seq().unwrap_or(0),
                                payload.metric_count()
                            );
                        }
                    }

                    MessageType::DDeath => {
                        println!(
                            "{} DDEATH from {}/{}",
                            log_prefix_fn(None),
                            edge_node_id,
                            topic.device_id().unwrap_or("?")
                        );
                    }

                    _ => {}
                }
            }
        }
    }

    fn request_rebirth_for_known_nodes(&mut self) {
        let stats = self.node_stats.lock().unwrap();

        if stats.is_empty() {
            println!(
                "{} No known nodes yet, will request rebirth as nodes are discovered",
                self.log_prefix(None)
            );
            return;
        }

        let awake_count = stats.values().filter(|s| s.online()).count();
        let sleeping_count = stats.values().filter(|s| s.sleeping()).count();
        let unknown_count = stats
            .values()
            .filter(|s| s.state == NodeSleepState::Unknown)
            .count();

        println!(
            "{} Requesting rebirth from {} known nodes (Sparkplug B 2.2 PRIMARY behavior)",
            self.log_prefix(None),
            stats.len()
        );
        println!(
            "  Awake: {}, Sleeping: {}, Unknown: {}",
            awake_count, sleeping_count, unknown_count
        );

        let node_ids: Vec<String> = stats.keys().cloned().collect();
        drop(stats); // Release lock before calling send_rebirth_command

        for node_id in node_ids {
            self.send_rebirth_command(&node_id);
        }
    }

    fn check_sleeping_nodes(&mut self) {
        let now = Instant::now();
        let mut nodes_to_wake = Vec::new();

        {
            let mut stats = self.node_stats.lock().unwrap();
            for (node_id, node_stats) in stats.iter_mut() {
                if !node_stats.sleeping() && !node_stats.wake_pending() {
                    continue;
                }

                let backoff_seconds = std::cmp::min(60, 5 * (1 << node_stats.wake_attempt_count));

                if let Some(last_attempt) = node_stats.last_wake_attempt {
                    let time_since_last = now.duration_since(last_attempt).as_secs();

                    if time_since_last >= backoff_seconds as u64 {
                        if node_stats.wake_pending() {
                            println!(
                                "{} Node {} did not respond to wake attempt #{} (waited {}s), back to SLEEPING",
                                self.log_prefix(None),
                                node_id,
                                node_stats.wake_attempt_count,
                                time_since_last
                            );
                            node_stats.state = NodeSleepState::Sleeping;
                        }

                        if node_stats.sleeping() {
                            nodes_to_wake.push((
                                node_id.clone(),
                                node_stats.wake_attempt_count,
                                backoff_seconds,
                            ));
                        }
                    }
                }
            }
        }

        for (node_id, attempt_count, backoff) in nodes_to_wake {
            println!(
                "{} Attempting to wake sleeping node {} (attempt #{}, backoff {}s)",
                self.log_prefix(None),
                node_id,
                attempt_count + 1,
                backoff
            );
            self.send_rebirth_command(&node_id);

            let mut stats = self.node_stats.lock().unwrap();
            if let Some(node_stats) = stats.get_mut(&node_id) {
                node_stats.state = NodeSleepState::WakePending;
                node_stats.last_wake_attempt = Some(now);
                node_stats.wake_attempt_count += 1;
            }
        }
    }

    fn send_rebirth_command(&mut self, edge_node_id: &str) {
        let log_prefix = self.log_prefix(None);

        if let Some(publisher) = &mut self.command_publisher {
            println!("{} Sending REBIRTH command to {}", log_prefix, edge_node_id);

            if let Ok(mut cmd) = PayloadBuilder::new() {
                if cmd.add_bool("Node Control/Rebirth", true).is_ok() {
                    if let Ok(cmd_bytes) = cmd.serialize() {
                        if let Err(e) = publisher.publish_node_command(edge_node_id, &cmd_bytes) {
                            eprintln!("{} Failed to send rebirth command: {}", log_prefix, e);
                        }
                    }
                }
            }
        }
    }

    fn send_reboot_command(&mut self, edge_node_id: &str) {
        let log_prefix = self.log_prefix(None);

        if let Some(publisher) = &mut self.command_publisher {
            println!(
                "{} Sending REBOOT command to {} (will cause crash)",
                log_prefix, edge_node_id
            );

            if let Ok(mut cmd) = PayloadBuilder::new() {
                if cmd.add_bool("Node Control/Reboot", true).is_ok() {
                    if let Ok(cmd_bytes) = cmd.serialize() {
                        if let Err(e) = publisher.publish_node_command(edge_node_id, &cmd_bytes) {
                            eprintln!("{} Failed to send reboot command: {}", log_prefix, e);
                        }
                    }
                }
            }
        }
    }

    fn run(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        let mut last_cycle = Instant::now();
        let mut last_command = Instant::now();

        println!("{} Starting monitoring loop...", self.log_prefix(None));
        if self.cycle_interval_sec > 0 {
            println!(
                "{} Will cycle connection every {} seconds",
                self.log_prefix(None),
                self.cycle_interval_sec
            );
        }
        println!();

        while RUNNING.load(Ordering::SeqCst) {
            let now = Instant::now();

            self.check_sleeping_nodes();

            if self.send_commands && now.duration_since(last_command).as_secs() >= 15 {
                let action = rng.random_range(0..100);

                let node_stats = self.node_stats.lock().unwrap();
                if action < 30 && !node_stats.is_empty() {
                    // Pick random online node for rebirth
                    let online_nodes: Vec<_> = node_stats
                        .iter()
                        .filter(|(_, stats)| stats.online())
                        .map(|(id, _)| id.clone())
                        .collect();

                    if !online_nodes.is_empty() {
                        let node_id = &online_nodes[rng.random_range(0..online_nodes.len())];
                        drop(node_stats);
                        self.send_rebirth_command(node_id);
                    }
                } else if action < 35 && !node_stats.is_empty() {
                    // Pick random online node for reboot
                    let online_nodes: Vec<_> = node_stats
                        .iter()
                        .filter(|(_, stats)| stats.online())
                        .map(|(id, _)| id.clone())
                        .collect();

                    if !online_nodes.is_empty() {
                        let node_id = online_nodes[rng.random_range(0..online_nodes.len())].clone();
                        drop(node_stats);
                        self.send_reboot_command(&node_id);
                    }
                }

                last_command = now;
            }

            if self.cycle_interval_sec > 0
                && now.duration_since(last_cycle).as_secs() >= self.cycle_interval_sec as u64
            {
                println!("\n{} *** CYCLING CONNECTION ***", self.log_prefix(None));
                let _ = self.disconnect();
                thread::sleep(Duration::from_secs(2));
                self.reconnect_count.fetch_add(1, Ordering::SeqCst);
                if let Err(e) = self.connect() {
                    eprintln!("{} Reconnection failed: {}", self.log_prefix(None), e);
                    break;
                }
                last_cycle = Instant::now();
                println!();
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Disconnecting...", self.log_prefix(None));

        if let Some(mut subscriber) = self.subscriber.take() {
            if let Err(e) = subscriber.disconnect() {
                eprintln!(
                    "{} Subscriber disconnect failed: {}",
                    self.log_prefix(None),
                    e
                );
            }
        }

        if let Some(mut publisher) = self.command_publisher.take() {
            if let Err(e) = publisher.disconnect() {
                eprintln!(
                    "{} Command publisher disconnect failed: {}",
                    self.log_prefix(None),
                    e
                );
            }
        }

        Ok(())
    }

    fn print_statistics(&self) {
        println!("\n{} Statistics:", self.log_prefix(None));
        println!(
            "  Total messages received: {}",
            self.messages_received.load(Ordering::SeqCst)
        );
        println!(
            "  Sequence errors: {}",
            self.sequence_errors.load(Ordering::SeqCst)
        );
        println!(
            "  Reconnection count: {}",
            self.reconnect_count.load(Ordering::SeqCst)
        );
        println!("\n  Per-Node Statistics:");

        let stats = self.node_stats.lock().unwrap();
        for (node_id, node_stats) in stats.iter() {
            println!("    {}:", node_id);
            println!("      State: {}", node_stats.state.description());
            println!("      NBIRTH: {}", node_stats.birth_count);
            println!("      NDEATH: {}", node_stats.death_count);
            println!("      NDATA: {}", node_stats.data_count);
            println!("      Current bdSeq: {}", node_stats.current_bd_seq);
            println!("      Last seq: {}", node_stats.last_seq);
            println!("      Wake attempts: {}", node_stats.wake_attempt_count);
        }
    }
}
