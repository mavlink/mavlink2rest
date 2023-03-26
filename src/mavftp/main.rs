mod mavftp;
use mavftp::*;

mod controller;
use controller::*;

use mavlink::MavConnection;
use std::io::Read;
use std::str;

use num_traits::FromPrimitive;

use std::sync::{mpsc, Arc, Mutex};

fn main() {
    let target_system = 1; // Replace with the target system ID
    let target_component = 0; // Replace with the target component ID

    //let url = "udpout:192.168.0.43:14660";
    let url = "tcpout:0.0.0.0:5760";
    let mut vehicle = mavlink::connect(&url).unwrap();
    vehicle.set_protocol_version(mavlink::MavlinkVersion::V2);
    let receiver = Arc::new(vehicle);
    let sender = receiver.clone();

    let mut header = mavlink::MavHeader::default();
    header.system_id = 1;
    header.component_id = 0;

    let path = ".".to_string();

    let payload = MavlinkFtpPayload::new(
        1,
        0,
        MavlinkFtpOpcode::ListDirectory,
        MavlinkFtpOpcode::None,
        0,
        0,
        path.as_bytes().to_vec(),
    );

    let msg = mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
        mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
            target_network: 0,
            target_system,
            target_component,
            payload: payload.to_bytes(),
        },
    );

    sender.send(&header, &msg).expect("Failed to send message");
    // let mut files = Vec::new();
    let mut controller = Controller::new();
    while let Ok((_header, message)) = receiver.recv() {
        if let Some(payload) = controller.run() {
            sender.send(
                &header,
                &mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
                    mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
                        target_network: 0,
                        target_system: 1,
                        target_component: 1,
                        payload: payload.to_bytes(),
                    },
                ),
            );
        }
        if let mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(msg) = message {
            controller.parse_mavlink_message(&msg);
            /*
            let payload = msg.payload;
            let opcode = payload[3];
            dbg!(&opcode);

            let opcode = MavlinkFtpOpcode::from_u8(opcode).unwrap();
            match opcode {
                MavlinkFtpOpcode::Ack => {
                    let data_size = payload[4] as usize;
                    let data = &payload[12..12 + data_size];
                    dbg!(&data.len());
                    let entries: Vec<&[u8]> = data.split(|&byte| byte == 0).collect();

                    if entries.is_empty() {
                        break;
                    }

                    let mut offset = 0;
                    for entry in entries {
                        if entry.is_empty() {
                            continue;
                        }

                        offset += 1;

                        if let Ok(result) = parse_directory_entry(&String::from_utf8_lossy(entry)) {
                            files.push(result);
                        }
                    }
                }
                MavlinkFtpOpcode::Nak => {
                    let nak_code = MavlinkFtpNak::from_u8(payload[12]).unwrap();
                    println!("Error: {:#?}", nak_code);
                    break;
                }
                _ => {}
            }

            dbg!(&files);
            */
        }
    }
}
