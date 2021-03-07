use std::sync::{Arc, Mutex};

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
    //thread_rx_channel: std::sync::mpsc::Receiver<(mavlink::MavHeader, M)>,
}

impl<M: mavlink::Message> MAVLinkVehicle<M> {
    fn new(mavlink_connection_string: &str) -> Self {
        Self {
            vehicle: Arc::new(mavlink::connect(&mavlink_connection_string).unwrap()),
        }
    }
}

impl<M: 'static + mavlink::Message + std::fmt::Debug + From<mavlink::common::MavMessage>>
    MAVLinkVehicleHandle<M>
{
    pub fn new(connection_string: &str) -> Self {
        let mavlink_vehicle: Arc<Mutex<MAVLinkVehicle<M>>> = Arc::new(Mutex::new(
            MAVLinkVehicle::<M>::new(connection_string.clone()),
        ));

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

    pub fn send(&self) -> std::io::Result<()> {
        unreachable!();
        //self.mavlink_vehicle.lock().unwrap().vehicle.send()
    }
}

fn receive_message_loop<
    M: mavlink::Message + std::fmt::Debug + From<mavlink::common::MavMessage>,
>(
    mavlink_vehicle: Arc<Mutex<MAVLinkVehicle<M>>>,
) {
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

fn heartbeat_loop<M: mavlink::Message + From<mavlink::common::MavMessage>>(
    mavlink_vehicle: Arc<Mutex<MAVLinkVehicle<M>>>,
) {
    let mavlink_vehicle = mavlink_vehicle.as_ref().lock().unwrap();
    let vehicle = mavlink_vehicle.vehicle.clone();
    drop(mavlink_vehicle);

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("sending heartbeat");
        if let Err(error) = vehicle.as_ref().send_default(&heartbeat_message().into()) {
            println!("Failed to send heartbeat: {:?}", error);
        }
    }
}

pub fn heartbeat_message() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::common::MavType::MAV_TYPE_QUADROTOR, // TODO: Move this to something else
        autopilot: mavlink::common::MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
        base_mode: mavlink::common::MavModeFlag::empty(),
        system_status: mavlink::common::MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    })
}
