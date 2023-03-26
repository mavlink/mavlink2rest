use crate::mavftp::*;
use num_traits::FromPrimitive;

struct ScanningFolderStatus {
    path: String,
    offset: u8,
}

pub struct Controller {
    entries: Vec<EntryInfo>,
    status: Option<ScanningFolderStatus>,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            status: Some(ScanningFolderStatus {
                path: ".".into(),
                offset: 0,
            })
        }
    }

    pub fn run(&self) -> Option<MavlinkFtpPayload> {
        if let Some(status) = &self.status {
            return Some(MavlinkFtpPayload::new(
                1,
                0,
                MavlinkFtpOpcode::ListDirectory,
                MavlinkFtpOpcode::None,
                0,
                status.offset as u32,
                status.path.as_bytes().to_vec()
            ));
        }

        return None;
    }

    pub fn parse_mavlink_message(
        &mut self, message: &mavlink::common::FILE_TRANSFER_PROTOCOL_DATA,
    ) -> Option<mavlink::common::MavMessage> {
        let Some(mut status) = self.status.as_mut() else {
            return None;
        };

        let payload = &message.payload;
        let opcode = payload[3];

        let opcode = MavlinkFtpOpcode::from_u8(opcode).unwrap();
        match opcode {
            MavlinkFtpOpcode::Ack => {
                let data_size = payload[4] as usize;
                let data = &payload[12..12 + data_size];
                let entries: Vec<&[u8]> = data.split(|&byte| byte == 0).collect();

                if entries.is_empty() {
                    return None;
                }

                for entry in entries {
                    if entry.is_empty() {
                        continue;
                    }
                    status.offset += 1;

                    if let Ok(mut result) = parse_directory_entry(&String::from_utf8_lossy(entry)) {
                        result.name = format!("{}/{}", status.path, result.name);
                        self.entries.push(result);
                    }
                }

                if status.offset != 0 {
                    return Some(mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
                        mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
                            target_network: 0,
                            target_system: 1,
                            target_component: 1,
                            payload: MavlinkFtpPayload::new(
                                1,
                                0,
                                MavlinkFtpOpcode::ListDirectory,
                                MavlinkFtpOpcode::None,
                                0,
                                status.offset as u32,
                                status.path.as_bytes().to_vec()
                            ).to_bytes(),
                        },
                    ));
                }
            }
            MavlinkFtpOpcode::Nak => {
                let nak_code = MavlinkFtpNak::from_u8(payload[12]).unwrap();
                println!("Error: {:#?}", nak_code);

                match nak_code {
                    MavlinkFtpNak::EOF => {
                        // We finished the current scan
                        dbg!(&self.entries);
                        dbg!("Done!");
                        return None;
                    }
                    _ => {
                        // Something is wrong... but it'll deal with it in the same way
                        return None
                    },
                }
            }
            _ => {}
        }

        return None;
    }
}
