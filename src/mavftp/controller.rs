use std::path::PathBuf;

use crate::mavftp::*;
use num_traits::FromPrimitive;

enum OperationStatus {
    ScanningFolder(ScanningFolderStatus),
    OpeningFile(OpeningFileStatus),
    ReadingFile(ReadingFileStatus),
}

struct ScanningFolderStatus {
    path: String,
    offset: u8,
}

struct OpeningFileStatus {
    path: String,
}

struct ReadingFileStatus {
    path: String,
    offset: u32,
    file_size: u32,
    content: Vec<u8>,
}

pub struct Controller {
    entries: Vec<EntryInfo>,
    status: Option<OperationStatus>,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            status: None,
        }
    }

    pub fn list_directory(&mut self, path: String) {
        self.status = Some(OperationStatus::ScanningFolder(ScanningFolderStatus {
            path,
            offset: 0,
        }))
    }

    pub fn read_file(&mut self, path: String) {
        self.status = Some(OperationStatus::OpeningFile(OpeningFileStatus{path}));
    }

    pub fn run(&self) -> Option<MavlinkFtpPayload> {
        match &self.status {
            Some(OperationStatus::ScanningFolder(status)) => {
                return Some(MavlinkFtpPayload::new(
                    1,
                    0,
                    MavlinkFtpOpcode::ListDirectory,
                    MavlinkFtpOpcode::None,
                    0,
                    status.offset as u32,
                    status.path.as_bytes().to_vec(),
                ));
            }
            Some(OperationStatus::OpeningFile(status)) => {
                dbg!("run!");
                return Some(MavlinkFtpPayload::new(
                    1,
                    0,
                    MavlinkFtpOpcode::OpenFileRO,
                    MavlinkFtpOpcode::None,
                    0,
                    0,
                    status.path.as_bytes().to_vec(),
                ));
            },
            Some(OperationStatus::ReadingFile(status)) => {
                return Some(MavlinkFtpPayload::new(
                    1,
                    0,
                    MavlinkFtpOpcode::ReadFile,
                    MavlinkFtpOpcode::None,
                    0,
                    status.offset,
                    status.path.as_bytes().to_vec(),
                ));
            }
            None => return None,
        }
    }

    pub fn parse_mavlink_message(
        &mut self, message: &mavlink::common::FILE_TRANSFER_PROTOCOL_DATA,
    ) -> Option<mavlink::common::MavMessage> {
        let payload = &message.payload;
        let opcode = payload[3];

        let opcode = MavlinkFtpOpcode::from_u8(opcode).unwrap();

        match opcode {
            MavlinkFtpOpcode::Ack => {
                let payload = MavlinkFtpPayload::from_bytes(&payload).unwrap();
                dbg!(&payload);

                match &mut self.status {
                    Some(OperationStatus::ScanningFolder(status)) => {
                        let entries: Vec<&[u8]> = payload.data.split(|&byte| byte == 0).collect();

                        if entries.is_empty() {
                            return None;
                        }

                        for entry in entries {
                            if entry.is_empty() {
                                continue;
                            }
                            status.offset += 1;

                            if let Ok(mut result) =
                                parse_directory_entry(&String::from_utf8_lossy(entry))
                            {
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
                                        status.path.as_bytes().to_vec(),
                                    )
                                    .to_bytes(),
                                },
                            ));
                        }
                    }
                    Some(OperationStatus::OpeningFile(status)) => {
                        dbg!("open!");
                        if payload.size != 4 {
                            panic!("Wrong size");
                        }
                        let file_size = u32::from_le_bytes([
                            payload.data[0],
                            payload.data[1],
                            payload.data[2],
                            payload.data[3],
                        ]);
                        
                        self.status = Some(OperationStatus::ReadingFile(ReadingFileStatus {
                            path: status.path.clone(),
                            offset: 0,
                            file_size,
                            content: Vec::new(),
                        }));

                        return None;
                    },
                    Some(OperationStatus::ReadingFile(status)) => {
                        let chunk = &payload.data;
                        status.content.extend_from_slice(chunk);
                        status.offset += chunk.len() as u32;
                        dbg!(&status.offset);

                        if status.offset < status.file_size {
                            return Some(mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
                                mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
                                    target_network: 0,
                                    target_system: 1,
                                    target_component: 1,
                                    payload: MavlinkFtpPayload::new(
                                        1,
                                        0,
                                        MavlinkFtpOpcode::ReadFile,
                                        MavlinkFtpOpcode::None,
                                        0,
                                        status.offset,
                                        status.path.as_bytes().to_vec(),
                                    )
                                    .to_bytes(),
                                },
                            ));
                        } else {
                            println!("File content: {:?}", std::str::from_utf8(&status.content));
                            self.status = None;
                            return None;
                        }
                    }
                    None => return None,
                }
            }
            MavlinkFtpOpcode::Nak => {
                let nak_code = MavlinkFtpNak::from_u8(payload[12]).unwrap();
                println!("Error: {:#?}", nak_code);

                match nak_code {
                    MavlinkFtpNak::EOF => {
                        // We finished the current operation
                        dbg!(&self.entries);
                        self.status = None;
                        return None;
                    }
                    _ => {
                        // Something is wrong... but it'll deal with it in the same way
                        return None;
                    }
                }
            }
            _ => {}
        }

        return None;
    }
}
