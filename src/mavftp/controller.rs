use std::process::exit;
use std::time::{Duration, SystemTime};
use std::{io::Write, path::PathBuf};

use crate::mavftp::*;
use num_traits::FromPrimitive;

use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use sha1::{Digest, Sha1};

use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom};

enum OperationStatus {
    ScanningFolder(ScanningFolderStatus),
    OpeningFile(OpeningFileStatus),
    ReadingFile(ReadingFileStatus),
    Reset,
    CalcFileCRC32(CalcFileCRC32Status)
}

struct ScanningFolderStatus {
    path: String,
    offset: u8,
}

struct OpeningFileStatus {
    path: String,
}

struct CalcFileCRC32Status {
    path: String,
}

struct ReadingFileStatus {
    path: String,
    offset: u32,
    file_size: u32,
    file: std::fs::File,
}

pub struct Controller {
    session: u8,
    last_time: SystemTime,
    entries: Vec<EntryInfo>,
    status: Option<OperationStatus>,
    waiting: bool,
    progress: Option<ProgressBar>,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            session: 0,
            last_time: SystemTime::now(),
            entries: Vec::new(),
            status: None,
            waiting: false,
            progress: None,
        }
    }

    pub fn list_directory(&mut self, path: String) {
        self.status = Some(OperationStatus::ScanningFolder(ScanningFolderStatus {
            path,
            offset: 0,
        }))
    }

    pub fn read_file(&mut self, path: String) {
        self.status = Some(OperationStatus::OpeningFile(OpeningFileStatus { path }));
    }

    pub fn reset(&mut self) {
        self.status = Some(OperationStatus::Reset);
    }

    pub fn crc(&mut self, path: String) {
        self.status = Some(OperationStatus::CalcFileCRC32(CalcFileCRC32Status { path }));
    }

    pub fn run(&mut self) -> Option<MavlinkFtpPayload> {
        /*
        if self.last_time.elapsed().unwrap() > Duration::from_millis(2) {
            self.last_time = SystemTime::now();
            self.waiting = false;
        }
         */
        if self.waiting {
            return None;
        }
        self.waiting = true;
        match &self.status {
            Some(OperationStatus::Reset) => {
                return Some(MavlinkFtpPayload::newResetSesions(
                    1,
                    self.session,
                ));
            }
            Some(OperationStatus::ScanningFolder(status)) => {
                return Some(MavlinkFtpPayload::newListDirectory(
                    1,
                    self.session,
                    status.offset as u32,
                    &status.path,
                ));
            }
            Some(OperationStatus::OpeningFile(status)) => {
                return Some(MavlinkFtpPayload::newOpenFile(
                    1,
                    self.session,
                    &status.path,
                ));
            }
            Some(OperationStatus::CalcFileCRC32(status)) => {
                return Some(MavlinkFtpPayload::newCalcFileCRC32(
                    1,
                    self.session,
                    &status.path,
                ));
            }
            Some(OperationStatus::ReadingFile(status)) => {
                return Some(MavlinkFtpPayload::newReadFile(
                    1,
                    self.session,
                    0,
                    usize::MAX,
                ));
            }
            None => return None,
        }
    }

    pub fn parse_mavlink_message(
        &mut self,
        message: &mavlink::common::FILE_TRANSFER_PROTOCOL_DATA,
    ) -> Option<mavlink::common::MavMessage> {
        self.waiting = false;
        let payload = &message.payload;
        let opcode = payload[3];

        let opcode = MavlinkFtpOpcode::from_u8(opcode).unwrap();

        match opcode {
            MavlinkFtpOpcode::Ack => {
                let payload = MavlinkFtpPayload::from_bytes(&payload).unwrap();

                match &mut self.status {
                    Some(OperationStatus::Reset) => {
                        if payload.req_opcode == MavlinkFtpOpcode::ResetSessions {
                            self.waiting = false;
                            self.status = None;
                        }
                    }
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
                            self.waiting = true;
                            return Some(mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
                                mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
                                    target_network: 0,
                                    target_system: 1,
                                    target_component: 1,
                                    payload: MavlinkFtpPayload::newListDirectory(
                                        1,
                                        self.session,
                                        status.offset as u32,
                                        &status.path,
                                    ).to_bytes(),
                                },
                            ));
                        }
                    }
                    Some(OperationStatus::OpeningFile(status)) => {
                        if payload.size != 4 {
                            panic!("Wrong size");
                        }
                        let file_size = u32::from_le_bytes([
                            payload.data[0],
                            payload.data[1],
                            payload.data[2],
                            payload.data[3],
                        ]);

                        self.progress = Some(ProgressBar::new(file_size as u64));
                        if let Some(progress) = &mut self.progress {
                            progress.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                                .unwrap()
                                .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
                                .progress_chars("#>-")
                            );
                        }

                        self.status = Some(OperationStatus::ReadingFile(ReadingFileStatus {
                            path: status.path.clone(),
                            offset: 0,
                            file_size,
                            file: OpenOptions::new()
                                .write(true)
                                .create(true)
                                .open("/tmp/potato2").unwrap(),
                        }));

                        return None;
                    }
                    Some(OperationStatus::CalcFileCRC32(status)) => {
                        if payload.req_opcode == MavlinkFtpOpcode::CalcFileCRC32 {
                            let crc = u32::from_le_bytes([
                                payload.data[0],
                                payload.data[1],
                                payload.data[2],
                                payload.data[3],
                            ]);
                            println!("0x{:x?}", crc);
                            exit(0);
                        }
                    },
                    Some(OperationStatus::ReadingFile(status)) => {
                        let chunk = &payload.data;
                        status.file.seek(SeekFrom::Start(payload.offset.into())).unwrap();
                        status.file.write_all(chunk).unwrap();
                        status.offset = payload.offset + payload.size as u32;
                        if let Some(progress) = &self.progress {
                            progress.set_position(status.offset as u64);
                        }

                        if payload.burst_complete == 1 {
                            self.waiting = true;
                            return Some(mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
                                mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
                                    target_network: 0,
                                    target_system: 1,
                                    target_component: 1,
                                    payload: MavlinkFtpPayload::newReadFile(
                                        payload.seq_number + 1,
                                        self.session,
                                        status.offset,
                                        usize::MAX,
                                    ).to_bytes(),
                                },
                            ));
                        }

                        if status.offset < status.file_size {
                            self.waiting = true;
                            return None;
                            /*
                            return Some(mavlink::common::MavMessage::FILE_TRANSFER_PROTOCOL(
                                mavlink::common::FILE_TRANSFER_PROTOCOL_DATA {
                                    target_network: 0,
                                    target_system: 1,
                                    target_component: 1,
                                    payload: MavlinkFtpPayload::new(
                                        1,
                                        self.session,
                                        MavlinkFtpOpcode::BurstReadFile,
                                        MavlinkFtpOpcode::None,
                                        0,
                                        status.offset,
                                        status.path.as_bytes().to_vec(),
                                    )
                                    .to_bytes(),
                                },
                            ));
                             */
                        } else {
                            //std::io::stdout().write_all(&status.content).unwrap();
                            self.waiting = false;
                            dbg!("Done!!");
                            //let mut hasher = Sha1::new();
                            //dbg!(&status.content.len());
                            //hasher.update(&status.content);
                            //println!("{:x?}", hasher.finalize());
                            //let mut f = std::fs::File::create("/tmp/potato").ok().unwrap();
                            //f.write_all(&status.content);
                            self.status = None;
                            return None;
                        }
                    }
                    None => return None,
                }
            }
            MavlinkFtpOpcode::Nak => {
                let nak_code = MavlinkFtpNak::from_u8(payload[12]).unwrap();

                match nak_code {
                    MavlinkFtpNak::EOF => {
                        exit(0);
                        // We finished the current operation
                        dbg!(&self.entries);
                        self.status = None;
                        return None;
                    }
                    MavlinkFtpNak::FailErrno => {
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
