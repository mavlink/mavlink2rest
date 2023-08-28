use std::sync::{mpsc, Arc, Mutex};

use log::*;

pub type MAVLinkVehicleArcMutex = Arc<Mutex<MAVLinkVehicle>>;

#[derive(Clone)]
pub struct MAVLinkVehicle {
    //TODO: Check if Arc<Box can be only Arc or Box
    vehicle: Arc<Box<dyn mavlink::MavConnection<mavlink::ardupilotmega::MavMessage> + Sync + Send>>,
    header: Arc<Mutex<mavlink::MavHeader>>,
}

impl MAVLinkVehicle {
    pub fn send(
        &self,
        header: &mavlink::MavHeader,
        message: &mavlink::ardupilotmega::MavMessage,
    ) -> std::io::Result<usize> {
        let result = self.vehicle.send(&header, &message);

        // Convert from mavlink error to io error
        match result {
            Err(mavlink::error::MessageWriteError::Io(error)) => Err(error),
            Ok(something) => Ok(something),
        }
    }
}

pub struct MAVLinkVehicleHandle {
    //TODO: Check if we can use vehicle here directly
    pub mavlink_vehicle: Arc<Mutex<MAVLinkVehicle>>,
    heartbeat_thread: std::thread::JoinHandle<()>,
    receive_message_thread: std::thread::JoinHandle<()>,
    //TODO: Add a channel for errors
    pub thread_rx_channel:
        std::sync::mpsc::Receiver<(mavlink::MavHeader, mavlink::ardupilotmega::MavMessage)>,
}

impl MAVLinkVehicle {
    fn new(
        mavlink_connection_string: &str,
        version: mavlink::MavlinkVersion,
        system_id: u8,
        component_id: u8,
    ) -> Self {
        let mut vehicle = mavlink::connect(mavlink_connection_string).unwrap();
        vehicle.set_protocol_version(version);
        let header = mavlink::MavHeader {
            system_id,
            component_id,
            sequence: 0,
        };

        Self {
            vehicle: Arc::new(vehicle),
            header: Arc::new(Mutex::new(header)),
        }
    }
}

impl MAVLinkVehicleHandle {
    pub fn new(
        connection_string: &str,
        version: mavlink::MavlinkVersion,
        system_id: u8,
        component_id: u8,
    ) -> Self {
        let mavlink_vehicle: Arc<Mutex<MAVLinkVehicle>> = Arc::new(Mutex::new(
            MAVLinkVehicle::new(connection_string.clone(), version, system_id, component_id),
        ));

        let heartbeat_mavlink_vehicle = mavlink_vehicle.clone();
        let receive_message_mavlink_vehicle = mavlink_vehicle.clone();

        let (tx_channel, rx_channel) =
            mpsc::channel::<(mavlink::MavHeader, mavlink::ardupilotmega::MavMessage)>();

        Self {
            mavlink_vehicle,
            heartbeat_thread: std::thread::spawn(move || heartbeat_loop(heartbeat_mavlink_vehicle)),
            receive_message_thread: std::thread::spawn(move || {
                receive_message_loop(receive_message_mavlink_vehicle, tx_channel);
            }),
            thread_rx_channel: rx_channel,
        }
    }
}

fn receive_message_loop(
    mavlink_vehicle: Arc<Mutex<MAVLinkVehicle>>,
    channel: std::sync::mpsc::Sender<(mavlink::MavHeader, mavlink::ardupilotmega::MavMessage)>,
) {
    let mavlink_vehicle = mavlink_vehicle.as_ref().lock().unwrap();

    let vehicle = mavlink_vehicle.vehicle.clone();
    drop(mavlink_vehicle);
    let vehicle = vehicle.as_ref();
    loop {
        match vehicle.recv() {
            Ok((header, msg)) => {
                if let Err(error) = channel.send((header, msg)) {
                    error!("Failed to send message though channel: {:#?}", error);
                }
            }
            Err(error) => {
                error!("Recv error: {:?}", error);
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

fn heartbeat_loop(mavlink_vehicle: Arc<Mutex<MAVLinkVehicle>>) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let mavlink_vehicle = mavlink_vehicle.as_ref().lock().unwrap();
        let vehicle = mavlink_vehicle.vehicle.clone();
        let mut header = mavlink_vehicle.header.lock().unwrap();
        if let Err(error) = vehicle.as_ref().send(&header, &heartbeat_message()) {
            error!("Failed to send heartbeat: {:?}", error);
        }
        header.sequence = header.sequence.wrapping_add(1);
    }
}

pub fn heartbeat_message() -> mavlink::ardupilotmega::MavMessage {
    mavlink::ardupilotmega::MavMessage::HEARTBEAT(mavlink::ardupilotmega::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::ardupilotmega::MavType::MAV_TYPE_ONBOARD_CONTROLLER,
        autopilot: mavlink::ardupilotmega::MavAutopilot::MAV_AUTOPILOT_INVALID,
        base_mode: mavlink::ardupilotmega::MavModeFlag::default(),
        system_status: mavlink::ardupilotmega::MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    })
}
