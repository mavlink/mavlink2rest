use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum_macros::{EnumIter, EnumString};

#[derive(Debug, Copy, Clone, PartialEq, EnumIter, FromPrimitive)]
pub enum MavlinkFtpOpcode {
    None = 0,
    TerminateSession = 1,
    ResetSessions = 2,
    ListDirectory = 3,
    OpenFileRO = 4,
    ReadFile = 5,
    CreateFile = 6,
    WriteFile = 7,
    RemoveFile = 8,
    CreateDirectory = 9,
    RemoveDirectory = 10,
    OpenFileWO = 11,
    TruncateFile = 12,
    Rename = 13,
    CalcFileCRC32 = 14,
    BurstReadFile = 15,
    Ack = 128,
    Nak = 129,
}

#[derive(Debug, Copy, Clone, PartialEq, EnumIter, EnumString, FromPrimitive)]
pub enum MavlinkFtpNak {
    #[strum(serialize = "No error")]
    None = 0,
    #[strum(serialize = "Unknown failure")]
    Fail = 1,
    #[strum(serialize = "Command failed, Err number sent back")]
    FailErrno = 2,
    #[strum(serialize = "Payload size is invalid")]
    InvalidDataSize = 3,
    #[strum(serialize = "Session is not currently open")]
    InvalidSession = 4,
    #[strum(serialize = "All available sessions are already in use")]
    NoSessionsAvailable = 5,
    #[strum(serialize = "Offset past end of file for ListDirectory and ReadFile commands")]
    EOF = 6,
    #[strum(serialize = "Unknown command / opcode")]
    UnknownCommand = 7,
    #[strum(serialize = "File/directory already exists")]
    FileExists = 8,
    #[strum(serialize = "File/directory is write protected")]
    FileProtected = 9,
    #[strum(serialize = "File/directory not found")]
    FileNotFound = 10,
}

#[derive(Debug)]
pub enum MavlinkFtpResponse {
    None,
    TerminateSession(u8),
    ResetSessions,
    ListDirectory(Vec<EntryInfo>),

    //OpenFileRO(u32, u32),
    //ReadFile(Vec<u8>),
    /*
    CreateFile(u32),
    WriteFile,
    RemoveFile,
    CreateDirectory,
    RemoveDirectory,
    OpenFileWO(u32),
    TruncateFile,
    Rename,
    CalcFileCRC32(u32),
    BurstReadFile(Vec<u8>),
     */
    Ack,
    Nak(MavlinkFtpNak),
}

#[derive(Debug)]
pub struct EntryInfo {
    pub entry_type: EntryType,
    pub name: String,
    pub size: u32,
}

#[derive(Debug)]
pub enum EntryType {
    File,
    Directory,
    Skip,
}

pub fn parse_directory_entry(entry: &str) -> Result<EntryInfo, &'static str> {
    let mut parts = entry.split('\t');
    let temp_filename = parts.next().unwrap();
    let file_type = temp_filename.chars().next();
    let name: String = temp_filename.chars().skip(1).collect();
    let size = parts.next().map(|s| s.parse().unwrap()).unwrap_or(0);

    let entry_type = match file_type {
        Some('F') => EntryType::File,
        Some('D') => EntryType::Directory,
        Some('S') => EntryType::Skip,
        _ => return Err("Invalid entry type"),
    };

    Ok(EntryInfo {
        entry_type,
        name,
        size,
    })
}

#[derive(Debug)]
pub struct MavlinkFtpPayload {
    // Sequence number for message (0 - 65535)
    pub seq_number: u16,
    // Session id for read/write operations (0 - 255)
    pub session: u8,
    // OpCode (id) for commands and ACK/NAK messages (0 - 255)
    pub opcode: MavlinkFtpOpcode,
    // Depends on OpCode. For Reads/Writes, it's the size of the data transported
    // For NAK, it's the number of bytes used for error information (1 or 2)
    pub size: usize,
    // OpCode (of original message) returned in an ACK or NAK response
    pub req_opcode: MavlinkFtpOpcode,
    // Code to indicate if a burst is complete (1: burst packets complete, 0: more burst packets coming)
    // Only used if req_opcode is BurstReadFile
    pub burst_complete: u8,
    // Padding for 32-bit alignment
    pub padding: u8,
    // Content offset for ListDirectory and ReadFile commands
    pub offset: u32,
    // Command/response data (varies by OpCode)
    pub data: Vec<u8>,
}

impl MavlinkFtpPayload {
    pub fn newResetSesions(
        seq_number: u16,
        session: u8,
    ) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::ResetSessions,
            size: 0,
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset: 0,
            data: vec![],
        }
    }

    pub fn newTerminateSession(
        seq_number: u16,
        session: u8,
    ) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::TerminateSession,
            size: 0,
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset: 0,
            data: vec![],
        }
    }

    pub fn newListDirectory(
        seq_number: u16,
        session: u8,
        offset: u32,
        path: &str,
    ) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::ListDirectory,
            size: path.len(),
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset,
            data: path.as_bytes().to_vec(),
        }
    }

    pub fn newOpenFile(
        seq_number: u16,
        session: u8,
        path: &str,
    ) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::OpenFileRO,
            size: path.len(),
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset: 0,
            data: path.as_bytes().to_vec(),
        }
    }

    pub fn newReadFile(
        seq_number: u16,
        session: u8,
        offset: u32,
        size_left: usize,
    ) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::BurstReadFile,
            size: size_left.clamp(0, 239), // 239 is the max size on the data field
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset,
            data: vec![],
        }
    }

    pub fn newCalcFileCRC32(
        seq_number: u16,
        session: u8,
        path: &str,
    ) -> Self {
    Self {
        seq_number,
        session,
        opcode: MavlinkFtpOpcode::CalcFileCRC32,
        size: path.len(),
        req_opcode: MavlinkFtpOpcode::None,
        burst_complete: 0,
        padding: 0,
        offset: 0,
        data: path.as_bytes().to_vec(),
    }
}

    /*
    opcode: MavlinkFtpOpcode,
        req_opcode: MavlinkFtpOpcode,
        burst_complete: u8,
        offset: u32,
        data: Vec<u8>,
        */

    // Convert payload structure into a byte array
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.seq_number.to_le_bytes());
        bytes.push(self.session);
        bytes.push(self.opcode as u8);
        bytes.push(self.size as u8);
        bytes.push(self.req_opcode as u8);
        bytes.push(self.burst_complete);
        bytes.push(self.padding);
        bytes.extend_from_slice(&self.offset.to_le_bytes());
        bytes.extend_from_slice(&self.data);

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<MavlinkFtpPayload, Box<dyn std::error::Error>> {
        if bytes.len() < 12 {
            return Err("Insufficient bytes in input array".into());
        }

        Ok(MavlinkFtpPayload {
            seq_number: u16::from_le_bytes([bytes[0], bytes[1]]),
            session: bytes[2],
            opcode: MavlinkFtpOpcode::from_u8(bytes[3]).ok_or("Invalid opcode")?,
            size: bytes[4] as usize,
            req_opcode: MavlinkFtpOpcode::from_u8(bytes[5]).ok_or("Invalid req_opcode")?,
            burst_complete: bytes[6],
            padding: bytes[7],
            offset: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            data: bytes[12..12 + bytes[4] as usize].to_vec(),
        })
    }
}
