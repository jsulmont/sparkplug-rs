use sparkplug_rs::{
    Message, MetricAlias, PayloadBuilder, Publisher, PublisherConfig, Result, Subscriber,
    SubscriberConfig,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn timestamp() -> String {
    let now = chrono::Local::now();
    now.format("%H:%M:%S%.3f").to_string()
}

#[derive(Debug, Clone)]
struct NodeState {
    last_seen: SystemTime,
    last_seq: Option<u64>,
    metrics: HashMap<String, f64>,
    aliases: HashMap<MetricAlias, String>,
    online: bool,
}

impl NodeState {
    fn new() -> Self {
        Self {
            last_seen: SystemTime::now(),
            last_seq: None,
            metrics: HashMap::new(),
            aliases: HashMap::new(),
            online: false,
        }
    }
}

type NodeMap = Arc<Mutex<HashMap<String, NodeState>>>;

fn main() -> Result<()> {
    println!("OT Subscriber - Monitoring Tool");
    println!("================================\n");

    // Get timestamp for STATE messages (must be consistent for birth and death)
    let state_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_millis() as u64;

    // Generate unique instance ID for MQTT client IDs (prevents collision when running multiple instances)
    let instance_id = state_timestamp % 100000; // Use last 5 digits of timestamp

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .expect("Error setting Ctrl-C handler");

    let nodes: NodeMap = Arc::new(Mutex::new(HashMap::new()));

    let nodes_clone = nodes.clone();
    let vpp_r2_config = SubscriberConfig::new(
        "tcp://localhost:1883",
        format!("ot_monitor_r2_{}", instance_id),
        "VPP_R2",
    );
    let mut vpp_r2_sub = Subscriber::new(
        vpp_r2_config,
        Box::new(move |msg: Message| {
            handle_message(&msg, &nodes_clone, "VPP_R2");
        }),
    )?;
    vpp_r2_sub.connect()?;
    vpp_r2_sub.subscribe_all()?;
    println!("[{}] [OK] Subscribed to VPP_R2/#", timestamp());

    let nodes_clone2 = nodes.clone();
    let vpp4s_r2_config = SubscriberConfig::new(
        "tcp://localhost:1883",
        format!("ot_monitor_4s_{}", instance_id),
        "VPP4S_R2",
    );
    let mut vpp4s_r2_sub = Subscriber::new(
        vpp4s_r2_config,
        Box::new(move |msg: Message| {
            handle_message(&msg, &nodes_clone2, "VPP4S_R2");
        }),
    )?;
    vpp4s_r2_sub.connect()?;
    vpp4s_r2_sub.subscribe_all()?;
    println!("[{}] [OK] Subscribed to VPP4S_R2/#", timestamp());

    let cmd_pub_r2_config = PublisherConfig::new(
        "tcp://localhost:1883",
        format!("ot_monitor_cmd_r2_{}", instance_id),
        "VPP_R2",
        "MONITOR",
    );
    let mut cmd_pub_r2 = Publisher::new(cmd_pub_r2_config)?;
    cmd_pub_r2.connect()?;

    // Publish STATE birth for Host Application (Sparkplug B 2.2 spec)
    cmd_pub_r2.publish_state_birth("MONITOR", state_timestamp)?;
    println!(
        "[{}] [VPP_R2] Published STATE birth for MONITOR",
        timestamp()
    );

    let cmd_pub_4s_config = PublisherConfig::new(
        "tcp://localhost:1883",
        format!("ot_monitor_cmd_4s_{}", instance_id),
        "VPP4S_R2",
        "MONITOR",
    );
    let mut cmd_pub_4s = Publisher::new(cmd_pub_4s_config)?;
    cmd_pub_4s.connect()?;

    // Publish STATE birth for Host Application (Sparkplug B 2.2 spec)
    cmd_pub_4s.publish_state_birth("MONITOR", state_timestamp)?;
    println!(
        "[{}] [VPP4S_R2] Published STATE birth for MONITOR",
        timestamp()
    );

    println!("\nSending rebirth requests to known nodes...");
    send_rebirth_request(&mut cmd_pub_r2, "BAL01")?;
    send_rebirth_request(&mut cmd_pub_4s, "CBHS01")?;
    println!("Rebirth requests sent\n");

    println!("Monitoring messages (Ctrl+C to stop)\n");

    let mut counter = 0;

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(1));
        counter += 1;

        if counter % 30 == 0 {
            print_status(&nodes);
        }

        check_stale_data(&nodes);
    }

    println!("\n[{}] Shutting down...", timestamp());

    // Disconnect subscribers first
    println!("[{}] Disconnecting subscribers...", timestamp());
    vpp_r2_sub.disconnect()?;
    vpp4s_r2_sub.disconnect()?;

    // Publish STATE death for Host Applications (Sparkplug B 2.2 spec requirement)
    println!("[{}] Publishing STATE death messages...", timestamp());
    cmd_pub_r2.publish_state_death("MONITOR", state_timestamp)?;
    cmd_pub_4s.publish_state_death("MONITOR", state_timestamp)?;

    // Disconnect publishers
    cmd_pub_r2.disconnect()?;
    cmd_pub_4s.disconnect()?;

    println!("[{}] Disconnected gracefully", timestamp());

    Ok(())
}

fn send_rebirth_request(publisher: &mut Publisher, node: &str) -> Result<()> {
    let mut payload = PayloadBuilder::new()?;
    payload.add_bool("Node Control/Rebirth", true)?;
    let payload_bytes = payload.serialize()?;

    publisher.publish_node_command(node, &payload_bytes)?;
    println!("[{}]   → Sent rebirth request to {}", timestamp(), node);
    Ok(())
}

fn handle_message(msg: &Message, nodes: &NodeMap, group: &str) {
    if let Ok(topic) = msg.parse_topic() {
        if let Some(msg_type) = topic.message_type() {
            if let Some(node_id) = topic.edge_node_id() {
                let key = format!("{}/{}", group, node_id);
                let mut nodes_map = nodes.lock().unwrap();
                let node = nodes_map.entry(key.clone()).or_insert_with(NodeState::new);
                node.last_seen = SystemTime::now();

                if msg_type.is_birth() {
                    let device = topic.device_id().unwrap_or("NODE");
                    println!("[{}] [{}] {} - BIRTH", timestamp(), key, device);
                    node.online = true;

                    if let Ok(payload) = msg.parse_payload() {
                        if let Some(seq) = payload.seq() {
                            node.last_seq = Some(seq);
                        }

                        for metric in payload.metrics().flatten() {
                            if let Some(name) = &metric.name {
                                if let Some(alias) = metric.alias {
                                    node.aliases.insert(alias, name.clone());
                                }
                                if let Some(val) = extract_double(&metric.value) {
                                    node.metrics.insert(name.clone(), val);
                                }
                            }
                        }
                    }
                } else if msg_type.is_data() {
                    if let Ok(payload) = msg.parse_payload() {
                        if let Some(seq) = payload.seq() {
                            if let Some(last_seq) = node.last_seq {
                                // Sparkplug B sequence numbers are 0-255 (wraps at 256)
                                let expected_seq = (last_seq + 1) % 256;
                                if seq != last_seq && seq != expected_seq {
                                    println!(
                                        "[{}] [{}] SEQUENCE GAP: expected {}, got {}",
                                        timestamp(),
                                        key,
                                        expected_seq,
                                        seq
                                    );
                                }
                            }
                            node.last_seq = Some(seq);
                        }

                        for metric in payload.metrics().flatten() {
                            let metric_name = if let Some(name) = &metric.name {
                                Some(name.clone())
                            } else if let Some(alias) = metric.alias {
                                node.aliases.get(&alias).cloned()
                            } else {
                                None
                            };

                            if let Some(name) = metric_name {
                                if let Some(val) = extract_double(&metric.value) {
                                    node.metrics.insert(name, val);
                                }
                            }
                        }
                    }
                } else if msg_type.is_death() {
                    println!("[{}] [{}] NODE DEATH", timestamp(), key);
                    node.online = false;
                }
            }
        }
    }
}

fn extract_double(value: &sparkplug_rs::MetricValue) -> Option<f64> {
    match value {
        sparkplug_rs::MetricValue::Double(v) => Some(*v),
        sparkplug_rs::MetricValue::Float(v) => Some(*v as f64),
        sparkplug_rs::MetricValue::Int64(v) => Some(*v as f64),
        sparkplug_rs::MetricValue::Int32(v) => Some(*v as f64),
        _ => None,
    }
}

fn print_status(nodes: &NodeMap) {
    let nodes_map = nodes.lock().unwrap();
    if nodes_map.is_empty() {
        println!("\n[{}] [STATUS] No nodes detected", timestamp());
        return;
    }

    println!("\n[{}] === Node Status ===", timestamp());
    for (key, state) in nodes_map.iter() {
        // Skip MONITOR nodes (they use STATE messages, not Sparkplug NBIRTH/NDATA)
        if key.ends_with("/MONITOR") {
            continue;
        }

        let age = SystemTime::now()
            .duration_since(state.last_seen)
            .unwrap_or(Duration::from_secs(0));

        print!("  {}: ", key);
        if !state.online {
            print!("OFFLINE ({:.0}s ago) ", age.as_secs());
        } else if age.as_secs() > 60 {
            print!("STALE ({:.0}s ago) ", age.as_secs());
        } else {
            print!("ACTIVE ");
        }

        if let Some(soc) = state.metrics.get("DATA/BESS_SOC_ACT") {
            print!("SOC={:.1}% ", soc);
        }
        if let Some(power) = state.metrics.get("DATA/BESS_P_ACT") {
            print!("P={:.1}kW ", power);
        }
        if let Some(pv) = state.metrics.get("DATA/PV_P_ACT") {
            print!("PV={:.1}kW ", pv);
        }
        if let Some(seq) = state.last_seq {
            print!("seq={}", seq);
        }
        println!();
    }
    println!("  =========================\n");
}

fn check_stale_data(nodes: &NodeMap) {
    let nodes_map = nodes.lock().unwrap();
    for (key, state) in nodes_map.iter() {
        // Skip MONITOR nodes (they use STATE messages, not Sparkplug NBIRTH/NDATA)
        if key.ends_with("/MONITOR") {
            continue;
        }

        let age = SystemTime::now()
            .duration_since(state.last_seen)
            .unwrap_or(Duration::from_secs(0));

        if age.as_secs() > 120 {
            println!(
                "[{}] [WARNING] {} data is stale ({:.0}s)",
                timestamp(),
                key,
                age.as_secs()
            );
        }
    }
}
