use sparkplug_rs::{
    Message, PayloadBuilder, Publisher, PublisherConfig, Result, Subscriber, SubscriberConfig,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn timestamp() -> String {
    let now = chrono::Local::now();
    now.format("%H:%M:%S%.3f").to_string()
}

struct BatteryState {
    soc: f64,
    power: f64,
    power_setpoint: Option<f64>,
    control_enabled: bool,
    nominal_capacity_kwh: f64,
    nominal_power_kw: f64,
    pv_power: f64,
    pv_nominal_kw: f64,
}

impl BatteryState {
    fn new(nominal_capacity_kwh: f64, nominal_power_kw: f64, pv_nominal_kw: f64) -> Self {
        Self {
            soc: 50.0,
            power: 0.0,
            power_setpoint: None,
            control_enabled: false,
            nominal_capacity_kwh,
            nominal_power_kw,
            pv_power: 0.0,
            pv_nominal_kw,
        }
    }

    fn update(&mut self, dt_secs: f64) {
        if self.control_enabled {
            if let Some(sp) = self.power_setpoint {
                self.power = sp.clamp(-self.nominal_power_kw, self.nominal_power_kw);
            }
        } else {
            let secs_since_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let hour = (secs_since_epoch / 3600) % 24;
            let minute = (secs_since_epoch / 60) % 60;
            let time_of_day = hour as f64 + minute as f64 / 60.0;

            self.power = match hour {
                0..=5 => -50.0,
                6..=9 => -100.0,
                10..=14 => 80.0,
                15..=17 => 120.0,
                18..=20 => 60.0,
                _ => -30.0,
            };

            self.pv_power = if (6.0..18.0).contains(&time_of_day) {
                let t = (time_of_day - 12.0).abs() / 6.0;
                self.pv_nominal_kw * (1.0 - t * t) * 0.85
            } else {
                0.0
            };
        }

        let energy_delta_kwh = (self.power * dt_secs) / 3600.0;
        self.soc =
            (self.soc - (energy_delta_kwh / self.nominal_capacity_kwh * 100.0)).clamp(0.0, 100.0);
    }

    fn stored_energy_kwh(&self) -> f64 {
        self.nominal_capacity_kwh * self.soc / 100.0
    }

    fn charge_avail_kwh(&self) -> f64 {
        self.nominal_capacity_kwh * (100.0 - self.soc) / 100.0
    }

    fn discharge_avail_kwh(&self) -> f64 {
        self.stored_energy_kwh()
    }

    fn poc_power(&self) -> f64 {
        self.power + self.pv_power
    }
}

fn publish_birth(
    publisher: &mut Publisher,
    _device: &str,
    state: &BatteryState,
    group: &str,
    node: &str,
) -> Result<()> {
    // Publish NBIRTH with node-level metrics
    let mut nbirth = PayloadBuilder::new()?;
    nbirth
        .add_uint64("bdSeq", publisher.bd_seq())?
        .add_string("Properties/GroupID", group)?
        .add_string("Properties/NodeID", node)?
        .add_bool("Node Control/Rebirth", false)?;
    let nbirth_bytes = nbirth.serialize()?;
    publisher.publish_birth(&nbirth_bytes)?;

    println!(
        "[{}] [{}/{}] Published NBIRTH (bdSeq={}, seq={})",
        timestamp(),
        group,
        node,
        publisher.bd_seq(),
        publisher.seq()
    );

    // Publish device births
    publish_device_births(publisher, state, node)?;

    Ok(())
}

fn publish_device_births(
    publisher: &mut Publisher,
    state: &BatteryState,
    node: &str,
) -> Result<()> {
    let mut poc_birth = PayloadBuilder::new()?;
    poc_birth
        .add_double_with_alias("DATA/POC_P_ACT", 100, state.poc_power())?
        .add_double_with_alias("DATA/POC_Q_ACT", 101, 0.0)?
        .add_double_with_alias("DATA/POC_V_ACT", 102, 230.0)?
        .add_double_with_alias("DATA/POC_F_ACT", 103, 50.0)?
        .add_bool_with_alias("DATA/POC_METER_AVAIL", 104, true)?;
    let poc_bytes = poc_birth.serialize()?;
    publisher.publish_device_birth("POC", &poc_bytes)?;

    let mut bess_birth = PayloadBuilder::new()?;
    bess_birth
        .add_double_with_alias("DATA/BESS_SOC_ACT", 200, state.soc)?
        .add_double_with_alias("DATA/BESS_SOH_ACT", 201, 95.0)?
        .add_double_with_alias("DATA/BESS_E_ACT", 202, state.stored_energy_kwh())?
        .add_double_with_alias("DATA/BESS_E_CAP_AVAIL_ACT", 203, state.nominal_capacity_kwh)?
        .add_double_with_alias(
            "DATA/BESS_E_DISCHARGE_AVAIL_ACT",
            204,
            state.discharge_avail_kwh(),
        )?
        .add_double_with_alias(
            "DATA/BESS_E_CHARGE_AVAIL_ACT",
            205,
            state.charge_avail_kwh(),
        )?
        .add_bool_with_alias("DATA/BESS_AVAIL", 206, true)?
        .add_double_with_alias("DATA/BESS_P_ACT", 207, state.power)?
        .add_double_with_alias("DATA/BESS_Q_ACT", 208, 0.0)?
        .add_double_with_alias("DATA/BESS_P_NOM_ACT", 209, state.nominal_power_kw)?
        .add_double_with_alias("DATA/BESS_Q_NOM_ACT", 210, 0.0)?
        .add_double_with_alias("DATA/BESS_P_LIM_MAX_ACT", 211, state.nominal_power_kw)?
        .add_double_with_alias("DATA/BESS_P_LIM_MIN_ACT", 212, -state.nominal_power_kw)?;
    let bess_bytes = bess_birth.serialize()?;
    publisher.publish_device_birth("BESS", &bess_bytes)?;

    let mut pv_birth = PayloadBuilder::new()?;
    pv_birth
        .add_double_with_alias("DATA/PV_P_ACT", 300, state.pv_power)?
        .add_double_with_alias("DATA/PV_Q_ACT", 301, 0.0)?
        .add_double_with_alias("DATA/PV_P_NOM_ACT", 302, state.pv_nominal_kw)?
        .add_double_with_alias("DATA/PV_P_LIM_MAX_ACT", 303, state.pv_nominal_kw)?
        .add_bool_with_alias("DATA/PV_AVAIL", 304, true)?;
    let pv_bytes = pv_birth.serialize()?;
    publisher.publish_device_birth("PV", &pv_bytes)?;

    let mut ctrl_birth = PayloadBuilder::new()?;
    ctrl_birth
        .add_bool_with_alias("DATA/BESS_P_CTRL_MODE_EN_ACT", 400, state.control_enabled)?
        .add_double_with_alias(
            "DATA/BESS_P_CTRL_SP_ACT",
            401,
            state.power_setpoint.unwrap_or(0.0),
        )?;
    let ctrl_bytes = ctrl_birth.serialize()?;
    publisher.publish_device_birth("CONTROLLER", &ctrl_bytes)?;

    println!(
        "[{}] [{}] Published DBIRTH for POC, BESS, PV, CONTROLLER (seq={})",
        timestamp(),
        node,
        publisher.seq()
    );

    Ok(())
}

fn publish_data(
    publisher: &mut Publisher,
    state: &BatteryState,
    node: &str,
    verbose: bool,
) -> Result<()> {
    let mut poc_data = PayloadBuilder::new()?;
    poc_data.add_double_by_alias(100, state.poc_power());
    let poc_bytes = poc_data.serialize()?;
    publisher.publish_device_data("POC", &poc_bytes)?;

    let mut bess_data = PayloadBuilder::new()?;
    bess_data
        .add_double_by_alias(200, state.soc)
        .add_double_by_alias(202, state.stored_energy_kwh())
        .add_double_by_alias(204, state.discharge_avail_kwh())
        .add_double_by_alias(205, state.charge_avail_kwh())
        .add_double_by_alias(207, state.power);
    let bess_bytes = bess_data.serialize()?;
    publisher.publish_device_data("BESS", &bess_bytes)?;

    let mut pv_data = PayloadBuilder::new()?;
    pv_data.add_double_by_alias(300, state.pv_power);
    let pv_bytes = pv_data.serialize()?;
    publisher.publish_device_data("PV", &pv_bytes)?;

    if verbose {
        println!("[{}] [{}] Published DDATA (POC/BESS/PV)", timestamp(), node);
    }

    Ok(())
}

fn main() -> Result<()> {
    println!("OT Publisher - Community Battery Simulator");
    println!("===========================================\n");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .expect("Error setting Ctrl-C handler");

    let bal01_state = Arc::new(Mutex::new(BatteryState::new(500.0, 250.0, 100.0)));
    let cbhs01_state = Arc::new(Mutex::new(BatteryState::new(1000.0, 500.0, 300.0)));

    let bal01_config = PublisherConfig::new("tcp://localhost:1883", "ot_bal01", "VPP_R2", "BAL01");
    let mut bal01_pub = Publisher::new(bal01_config)?;
    bal01_pub.connect()?;

    let cbhs01_config =
        PublisherConfig::new("tcp://localhost:1883", "ot_cbhs01", "VPP4S_R2", "CBHS01");
    let mut cbhs01_pub = Publisher::new(cbhs01_config)?;
    cbhs01_pub.connect()?;

    let bal01_rebirth = Arc::new(AtomicBool::new(false));
    let cbhs01_rebirth = Arc::new(AtomicBool::new(false));

    let bal01_state_clone = bal01_state.clone();
    let cbhs01_state_clone = cbhs01_state.clone();
    let bal01_rebirth_clone = bal01_rebirth.clone();
    let cbhs01_rebirth_clone = cbhs01_rebirth.clone();

    let cmd_config = SubscriberConfig::new("tcp://localhost:1883", "ot_cmd_listener", "VPP_R2");
    let mut cmd_sub = Subscriber::new(
        cmd_config,
        Box::new(move |msg: Message| {
            if let Ok(topic) = msg.parse_topic() {
                if let Some(msg_type) = topic.message_type() {
                    if msg_type.is_command() {
                        if let Some(node) = topic.edge_node_id() {
                            if node == "BAL01" && msg_type.as_str() == "NCMD" {
                                println!(
                                    "[{}] [VPP_R2/BAL01] Received rebirth request",
                                    timestamp()
                                );
                                bal01_rebirth_clone.store(true, Ordering::SeqCst);
                            } else if node == "BAL01" && topic.device_id() == Some("CONTROLLER") {
                                handle_device_command(&msg, &bal01_state_clone, "BAL01");
                            }
                        }
                    }
                }
            }
        }),
    )?;
    cmd_sub.connect()?;
    cmd_sub.subscribe_all()?;

    let cmd_config2 = SubscriberConfig::new("tcp://localhost:1883", "ot_cmd_listener2", "VPP4S_R2");
    let mut cmd_sub2 = Subscriber::new(
        cmd_config2,
        Box::new(move |msg: Message| {
            if let Ok(topic) = msg.parse_topic() {
                if let Some(msg_type) = topic.message_type() {
                    if msg_type.is_command() {
                        if let Some(node) = topic.edge_node_id() {
                            if node == "CBHS01" && msg_type.as_str() == "NCMD" {
                                println!(
                                    "[{}] [VPP4S_R2/CBHS01] Received rebirth request",
                                    timestamp()
                                );
                                cbhs01_rebirth_clone.store(true, Ordering::SeqCst);
                            } else if node == "CBHS01" && topic.device_id() == Some("CONTROLLER") {
                                handle_device_command(&msg, &cbhs01_state_clone, "CBHS01");
                            }
                        }
                    }
                }
            }
        }),
    )?;
    cmd_sub2.connect()?;
    cmd_sub2.subscribe_all()?;

    publish_birth(
        &mut bal01_pub,
        "BAL01",
        &bal01_state.lock().unwrap(),
        "VPP_R2",
        "BAL01",
    )?;
    publish_birth(
        &mut cbhs01_pub,
        "CBHS01",
        &cbhs01_state.lock().unwrap(),
        "VPP4S_R2",
        "CBHS01",
    )?;

    println!("\nPublishing telemetry (Ctrl+C to stop)...\n");

    let mut counter = 0;
    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(5));
        counter += 1;

        if bal01_rebirth.swap(false, Ordering::SeqCst) {
            // First call rebirth() to increment bdSeq and publish NBIRTH
            bal01_pub.rebirth()?;
            // Then publish device births (DBIRTH messages)
            let state = bal01_state.lock().unwrap();
            publish_device_births(&mut bal01_pub, &state, "BAL01")?;
        }

        if cbhs01_rebirth.swap(false, Ordering::SeqCst) {
            // First call rebirth() to increment bdSeq and publish NBIRTH
            cbhs01_pub.rebirth()?;
            // Then publish device births (DBIRTH messages)
            let state = cbhs01_state.lock().unwrap();
            publish_device_births(&mut cbhs01_pub, &state, "CBHS01")?;
        }

        {
            let mut state = bal01_state.lock().unwrap();
            state.update(5.0);
            publish_data(&mut bal01_pub, &state, "VPP_R2/BAL01", counter % 6 == 0)?;
        }

        {
            let mut state = cbhs01_state.lock().unwrap();
            state.update(5.0);
            publish_data(&mut cbhs01_pub, &state, "VPP4S_R2/CBHS01", counter % 6 == 0)?;
        }

        if counter % 6 == 0 {
            let bal01 = bal01_state.lock().unwrap();
            let cbhs01 = cbhs01_state.lock().unwrap();
            println!(
                "[{}] Cycle {} | BAL01: SOC={:.1}% P={:.1}kW PV={:.1}kW | CBHS01: SOC={:.1}% P={:.1}kW PV={:.1}kW",
                timestamp(), counter, bal01.soc, bal01.power, bal01.pv_power, cbhs01.soc, cbhs01.power, cbhs01.pv_power
            );
        }
    }

    println!("\n[{}] Shutting down...", timestamp());

    // Disconnect subscribers first to stop callbacks
    println!("[{}] Disconnecting subscribers...", timestamp());
    cmd_sub.disconnect()?;
    cmd_sub2.disconnect()?;

    // Disconnect publishers - NDEATH will be sent via MQTT LWT automatically
    println!("[{}] Disconnecting publishers...", timestamp());
    bal01_pub.disconnect()?;
    cbhs01_pub.disconnect()?;

    println!("[{}] Disconnected gracefully", timestamp());

    Ok(())
}

fn handle_device_command(msg: &Message, state: &Arc<Mutex<BatteryState>>, node: &str) {
    if let Ok(payload) = msg.parse_payload() {
        // Use try_lock or handle lock errors gracefully during shutdown
        match state.lock() {
            Ok(mut state) => {
                for metric in payload.metrics().flatten() {
                    if let Some(name) = &metric.name {
                        match name.as_str() {
                            "CMD/BESS_P_CTRL_MODE_EN_CMD" => {
                                if let sparkplug_rs::MetricValue::Boolean(v) = metric.value {
                                    state.control_enabled = v;
                                    println!(
                                        "[{}] [{}] Control mode: {}",
                                        timestamp(),
                                        node,
                                        if v { "ENABLED" } else { "DISABLED" }
                                    );
                                }
                            }
                            "CMD/BESS_P_CTRL_SP" => {
                                if let sparkplug_rs::MetricValue::Double(v) = metric.value {
                                    state.power_setpoint = Some(v);
                                    println!(
                                        "[{}] [{}] Power setpoint: {:.1} kW",
                                        timestamp(),
                                        node,
                                        v
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {
                // Mutex lock failed (likely during shutdown) - silently ignore
            }
        }
    }
}
