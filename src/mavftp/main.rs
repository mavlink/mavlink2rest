mod mavftp;
use mavftp::*;

mod controller;
use controller::*;

use structopt::StructOpt;

mod cli;
use cli::*;

use std::{sync::Arc, thread, time::Duration};

fn main() {
    let target_system = 1;
    let target_component = 0;

    let mut header = mavlink::MavHeader::default();
    header.system_id = 1;
    header.component_id = 0;

    let args = cli::Opt::from_args();

    let url = args.connection;

    let mut vehicle = mavlink::connect(&url).unwrap();
    vehicle.set_protocol_version(mavlink::MavlinkVersion::V2);

    let receiver = Arc::new(vehicle);
    let sender = receiver.clone();

    thread::spawn({
        let vehicle = sender.clone();
        move || loop {
            let res = vehicle.send_default(&heartbeat_message());
            if res.is_ok() {
                thread::sleep(Duration::from_secs(1));
            }
            thread::sleep(Duration::from_secs(1));
        }
    });

    let mut controller = Controller::new();
    match args.command {
        MavlinkFTPCommand::ListDirectory { path } => controller.list_directory(path),
        MavlinkFTPCommand::ReadFile { path } => controller.read_file(path),
        MavlinkFTPCommand::Reset => controller.reset(),
        MavlinkFTPCommand::CalcFileCRC32 { path } => controller.crc(path),
        _ => panic!("Unsupported command!"),
    }

    loop {
        while let Ok((_header, message)) = receiver.recv() {
            if let Some(payload) = controller.run() {
                sender
                    .send(
                        &header,
                        &mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
                            mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
                                target_network: 0,
                                target_system,
                                target_component,
                                payload: payload.to_bytes(),
                            },
                        ),
                    )
                    .expect("Failed to send message");
            }

            if let mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(msg) = message {
                if let Some(msg) = controller.parse_mavlink_message(&msg) {
                    sender.send(&header, &msg).expect("Failed to send message");
                }
            }
        }
    }
}

pub fn heartbeat_message() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::common::MavType::MAV_TYPE_ONBOARD_CONTROLLER,
        autopilot: mavlink::common::MavAutopilot::MAV_AUTOPILOT_INVALID,
        base_mode: mavlink::common::MavModeFlag::default(),
        system_status: mavlink::common::MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    })
}
