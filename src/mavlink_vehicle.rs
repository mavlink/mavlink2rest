use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MAVLinkVehicle {
    vehicle: Arc<Box<dyn mavlink::MavConnection<mavlink::common::MavMessage> + Sync + Send>>,
}

pub struct MAVLinkVehicleHandle {
    mavlink_vehicle: Arc<Mutex<MAVLinkVehicle>>,
    heartbeat_thread: std::thread::JoinHandle<()>,
    receive_message_thread: std::thread::JoinHandle<()>,
}

impl MAVLinkVehicle {
    fn new(mavlink_connection_string: &str) -> Self {
        Self {
            vehicle: Arc::new(mavlink::connect(&mavlink_connection_string).unwrap()),
        }
    }
}

impl MAVLinkVehicleHandle {
    pub fn new(connection_string: &str) -> Self {
        let mavlink_vehicle: Arc<Mutex<MAVLinkVehicle>> =
            Arc::new(Mutex::new(MAVLinkVehicle::new(connection_string.clone())));

        let heartbeat_mavlink_vehicle = mavlink_vehicle.clone();
        let receive_message_mavlink_vehicle = mavlink_vehicle.clone();

        Self {
            mavlink_vehicle: mavlink_vehicle.clone(),
            heartbeat_thread: std::thread::spawn(move || heartbeat_loop(heartbeat_mavlink_vehicle)),
            receive_message_thread: std::thread::spawn(move || {
                receive_message_loop(receive_message_mavlink_vehicle)
            }),
        }
    }
}

fn heartbeat_loop(mavlink_vehicle: Arc<Mutex<MAVLinkVehicle>>) {
    let mavlink_vehicle = mavlink_vehicle.as_ref().lock().unwrap();
    let vehicle = mavlink_vehicle.vehicle.clone();
    drop(mavlink_vehicle);

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("sending heartbeat");
        if let Err(error) = vehicle.as_ref().send_default(&heartbeat_message()) {
            println!("Failed to send heartbeat: {:?}", error);
        }
    }
}

fn heartbeat_message() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::common::MavType::MAV_TYPE_GCS,
        autopilot: mavlink::common::MavAutopilot::MAV_AUTOPILOT_GENERIC,
        base_mode: mavlink::common::MavModeFlag::empty(),
        system_status: mavlink::common::MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    })
}

fn receive_message_loop(mavlink_vehicle: Arc<Mutex<MAVLinkVehicle>>) {
    let mavlink_vehicle = mavlink_vehicle.as_ref().lock().unwrap();

    let vehicle = mavlink_vehicle.vehicle.clone();
    drop(mavlink_vehicle);
    let vehicle = vehicle.as_ref();
    loop {
        match vehicle.recv() {
            Ok((_header, msg)) => {
                println!(">>> {:#?} {:#?}", _header, msg);
            }
            Err(error) => {
                println!("Recv error: {:?}", error);
                if let mavlink::error::MessageReadError::Io(error) = error {
                    if error.kind() == std::io::ErrorKind::UnexpectedEof {
                        // We're probably running a file, time to exit!
                        std::process::exit(0);
                    };
                }
            }
        }
    }
}
