mod mavftp;
use mavftp::*;

use mavlink::MavConnection;
use std::io::Read;
use std::str;

use num_traits::FromPrimitive;

fn main() {
    let target_system = 1; // Replace with the target system ID
    let target_component = 0; // Replace with the target component ID

    //let url = "udpout:192.168.0.46:14660";
    let url = "tcpout:0.0.0.0:5760";
    let mut conn = mavlink::connect(&url).unwrap();
    conn.set_protocol_version(mavlink::MavlinkVersion::V2);

    let mut payload = [0; 249];

    let mut header = mavlink::MavHeader::default();
    header.system_id = 1;
    header.component_id = 0;

    let path = ".\0".to_string();

    // Set the appropriate payload for the "List" operation
    let seq_number: u16 = 1;
    payload[0..2].copy_from_slice(&seq_number.to_le_bytes()); // Sequence number
    payload[2] = 0; // Session ID
    payload[3] = 3; // OpCode: 3 for "ListDirectory"
    payload[4] = 1; 
    payload[8..12].copy_from_slice(&0u32.to_le_bytes()); // Directory offset
    let path_bytes = path.as_bytes();
    payload[12..12 + path_bytes.len()].copy_from_slice(path_bytes); // Directory path to list files from

    let msg = mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
        mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
            target_network: 0,
            target_system,
            target_component,
            payload: payload.into(),
        },
    );

    let mut buf = [0; 300];

    dbg!("send");
    conn.send(&header, &msg).expect("Failed to send message");
    dbg!("loop");
    let mut files = Vec::new();
    while let Ok((header, message)) = conn.recv() {
        if let mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(msg) = message {
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

                        files.push(parse_directory_entry(&String::from_utf8_lossy(entry)));
                    }
                    dbg!(offset);
                }
                MavlinkFtpOpcode::Nak => {
                    let nak_code = MavlinkFtpNak::from_u8(payload[12]).unwrap();
                    println!("Error: {:#?}", nak_code);
                    break;
                }
                _ => {}
            }

            dbg!("files!");
            dbg!(&files);

            /*
            // Check if the received message is an ACK (0x80)
            if opcode == 0x80 {
                // Extract and print the list of files
                let file_list: Vec<String> = str::from_utf8(&payload[12..])
                    .unwrap_or("")
                    .split("\0")
                    .map(Into::into)
                    .collect();
                let file_list: Vec<&String> =
                    file_list.iter().filter(|&name| !name.is_empty()).collect();
                println!("List of files:\n{:?}", file_list);

                break;
            } else if opcode == 0x81 {
                // NAK response
                let error_code = payload[4];
                let error_description = match error_code {
                    0 => "None",
                    1 => "Fail",
                    2 => {
                        let errno = payload[5];
                        println!("FailErrno with error number: {}", errno);
                        "FailErrno"
                    }
                    3 => "InvalidDataSize",
                    4 => "InvalidSession",
                    5 => "NoSessionsAvailable",
                    6 => "EOF",
                    7 => "UnknownCommand",
                    8 => "FileExists",
                    9 => "FileProtected",
                    10 => "FileNotFound",
                    _ => "Unknown error",
                };
                println!(
                    "Received NAK with error code {}: {}",
                    error_code, error_description
                );
                break;
            }
             */
        }
    }
}
