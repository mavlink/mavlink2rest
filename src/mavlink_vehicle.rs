use std::sync::{mpsc, Arc, Mutex};

use log::*;
use mavlink;

#[derive(Clone)]
pub struct MAVLinkVehicle<M: mavlink::Message> {
    //TODO: Check if Arc<Box can be only Arc or Box
    vehicle: Arc<Box<dyn mavlink::MavConnection<M> + Sync + Send>>,
}

pub struct MAVLinkVehicleHandle<M: mavlink::Message> {
    //TODO: Check if we can use vehicle here directly
    mavlink_vehicle: Arc<Mutex<MAVLinkVehicle<M>>>,
    heartbeat_thread: std::thread::JoinHandle<()>,
    receive_message_thread: std::thread::JoinHandle<()>,
    //TODO: Add a channel for errors
    pub thread_rx_channel: std::sync::mpsc::Receiver<(mavlink::MavHeader, M)>,
}

impl<M: mavlink::Message> MAVLinkVehicle<M> {
    fn new(mavlink_connection_string: &str, version: mavlink::MavlinkVersion) -> Self {
        let mut vehicle = mavlink::connect(&mavlink_connection_string).unwrap();
        vehicle.set_protocol_version(version);
        Self {
            vehicle: Arc::new(vehicle),
        }
    }
}

impl<
        M: 'static + mavlink::Message + std::fmt::Debug + From<mavlink::common::MavMessage> + Send,
    > MAVLinkVehicleHandle<M>
{
    pub fn new(connection_string: &str, version: mavlink::MavlinkVersion) -> Self {
        let mavlink_vehicle: Arc<Mutex<MAVLinkVehicle<M>>> = Arc::new(Mutex::new(
            MAVLinkVehicle::<M>::new(connection_string.clone(), version),
        ));

        let heartbeat_mavlink_vehicle = mavlink_vehicle.clone();
        let receive_message_mavlink_vehicle = mavlink_vehicle.clone();

        let (tx_channel, rx_channel) = mpsc::channel::<(mavlink::MavHeader, M)>();

        Self {
            mavlink_vehicle: mavlink_vehicle.clone(),
            heartbeat_thread: std::thread::spawn(move || heartbeat_loop(heartbeat_mavlink_vehicle)),
            receive_message_thread: std::thread::spawn(move || {
                receive_message_loop(receive_message_mavlink_vehicle, tx_channel);
            }),
            thread_rx_channel: rx_channel,
        }
    }

    pub fn send(&self, header: &mavlink::MavHeader, message: &M) -> std::io::Result<()> {
        let result = self
            .mavlink_vehicle
            .lock()
            .unwrap()
            .vehicle
            .send(&header, &message);

        // Convert from mavlink error to io error
        match result {
            Err(mavlink::error::MessageWriteError::Io(error)) => Err(error),
            Ok(something) => Ok(something),
        }
    }
}

fn receive_message_loop<
    M: mavlink::Message + std::fmt::Debug + From<mavlink::common::MavMessage>,
>(
    mavlink_vehicle: Arc<Mutex<MAVLinkVehicle<M>>>,
    channel: std::sync::mpsc::Sender<(mavlink::MavHeader, M)>,
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

fn heartbeat_loop<M: mavlink::Message + From<mavlink::common::MavMessage>>(
    mavlink_vehicle: Arc<Mutex<MAVLinkVehicle<M>>>,
) {
    let mut counter: u8 = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        debug!("Sending heartbeat");
        let header = mavlink::MavHeader {
            system_id: 1,
            component_id: 1,
            sequence: counter,
        };
        counter = counter.wrapping_add(1);
        let mavlink_vehicle = mavlink_vehicle.as_ref().lock().unwrap();
        let vehicle = mavlink_vehicle.vehicle.clone();
        if let Err(error) = vehicle.as_ref().send(&header, &heartbeat_message().into()) {
            error!("Failed to send heartbeat: {:?}", error);
        }
    }
}

pub fn heartbeat_message() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::common::MavType::MAV_TYPE_GENERIC,
        autopilot: mavlink::common::MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
        base_mode: mavlink::common::MavModeFlag::default(),
        system_status: mavlink::common::MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    })
}
