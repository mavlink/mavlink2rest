use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::message_information::MessageInformation;

// TODO: Break this in shared and not mutex shared variables
pub struct InnerVehicle {
    pub channel: Arc<
        Box<
            (dyn mavlink::MavConnection<mavlink::ardupilotmega::MavMessage>
                 + std::marker::Send
                 + std::marker::Sync
                 + 'static),
        >,
    >,
    pub messages: Arc<Mutex<serde_json::value::Value>>,
    verbose: Arc<bool>,
    pub new_message_callback: Option<Arc<dyn Fn(&serde_json::Value, &String) + Send + Sync>>,
}

pub struct Vehicle {
    pub inner: Arc<Mutex<InnerVehicle>>,
}

impl Vehicle {
    // Move arguments to struct
    pub fn new(
        connection_string: &str,
        mavlink_version: mavlink::MavlinkVersion,
        verbose: bool,
    ) -> Vehicle {
        let mut mavlink_communication =
            mavlink::connect(connection_string).expect("Unable to connect!");
        mavlink_communication.set_protocol_version(mavlink_version);
        Vehicle {
            inner: Arc::new(Mutex::new(InnerVehicle {
                channel: Arc::new(mavlink_communication),
                messages: Arc::new(Mutex::new(json!({"mavlink":{}}))),
                verbose: Arc::new(verbose),
                new_message_callback: Default::default(),
            })),
        }
    }

    pub fn run(&mut self) {
        let inner = Arc::clone(&self.inner);
        let inner = inner.lock().unwrap();
        InnerVehicle::heartbeat_loop(&inner);
        InnerVehicle::parser_loop(&inner);
        let _ = inner.channel.send_default(&InnerVehicle::request_stream());
    }
}

impl InnerVehicle {
    fn heartbeat_loop(inner: &InnerVehicle) {
        let vehicle = inner.channel.clone();
        thread::spawn(move || loop {
            let res = vehicle.send_default(&InnerVehicle::heartbeat_message());
            if res.is_err() {
                println!("Failed to send heartbeat");
            }
            thread::sleep(Duration::from_secs(1));
        });
    }

    fn parser_loop(inner: &InnerVehicle) {
        let verbose = Arc::clone(&inner.verbose);
        let vehicle = inner.channel.clone();
        let messages_ref = Arc::clone(&inner.messages);
        let callback = match &inner.new_message_callback {
            Some(callback) => Some(Arc::clone(&callback)),
            _ => None,
        };

        let mut messages_information: std::collections::HashMap<
            std::string::String,
            MessageInformation,
        > = std::collections::HashMap::new();

        thread::spawn(move || loop {
            match vehicle.recv() {
                Ok((_header, msg)) => {
                    let value = serde_json::to_value(&msg).unwrap();
                    let mut msgs = messages_ref.lock().unwrap();
                    // Remove " from string
                    let msg_type = value["type"].to_string().replace("\"", "");
                    msgs["mavlink"][&msg_type] = value;
                    if *verbose {
                        println!("Got: {}", msg_type);
                    }

                    // Update message_information
                    let message_information = messages_information
                        .entry(msg_type.clone())
                        .or_insert_with(MessageInformation::default);
                    message_information.update();

                    msgs["mavlink"][&msg_type]["message_information"] =
                        serde_json::to_value(messages_information[&msg_type]).unwrap();

                    if callback.is_some() {
                        callback.as_ref().unwrap()(&msgs["mavlink"][&msg_type], &msg_type);
                    }
                }
                Err(error) => {
                    println!("recv error: {:?}", error);
                    if let mavlink::error::MessageReadError::Io(error) = error {
                        if error.kind() == std::io::ErrorKind::UnexpectedEof {
                            // We're probably running a file, time to exit!
                            std::process::exit(0);
                        };
                    }
                }
            }
        });
    }

    pub fn heartbeat_message() -> mavlink::ardupilotmega::MavMessage {
        mavlink::ardupilotmega::MavMessage::ardupilotmega({
            mavlink::ardupilotmega::MavMessage::HEARTBEAT(mavlink::ardupilotmega::HEARTBEAT_DATA {
                custom_mode: 0,
                mavtype: mavlink::ardupilotmega::MavType::MAV_TYPE_QUADROTOR, // TODO: Move this to something else
                autopilot: mavlink::ardupilotmega::MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
                base_mode: mavlink::ardupilotmega::MavModeFlag::empty(),
                system_status: mavlink::ardupilotmega::MavState::MAV_STATE_STANDBY,
                mavlink_version: 0x3,
            })
        })
    }

    pub fn request_stream() -> mavlink::ardupilotmega::MavMessage {
        mavlink::ardupilotmega::MavMessage::ardupilotmega({
            mavlink::ardupilotmega::MavMessage::REQUEST_DATA_STREAM(
                mavlink::ardupilotmega::REQUEST_DATA_STREAM_DATA {
                    target_system: 0,
                    target_component: 0,
                    req_stream_id: 0,
                    req_message_rate: 10,
                    start_stop: 1,
                },
            )
        })
    }
}
