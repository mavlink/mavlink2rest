use num_derive::FromPrimitive;
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
    TerminateSession(u8),
    ResetSessions,
    ListDirectory(Vec<FileInfo>),
    /*
    OpenFileRO(u32, u32),
    ReadFile(Vec<u8>),
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
pub struct FileInfo {
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

pub fn parse_directory_entry(entry: &str) -> Result<FileInfo, &'static str> {
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

    Ok(FileInfo {
        entry_type,
        name,
        size,
    })
}

pub struct MavlinkFtpPayload {
    // Sequence number for message (0 - 65535)
    pub seq_number: u16,
    // Session id for read/write operations (0 - 255)
    pub session: u8,
    // OpCode (id) for commands and ACK/NAK messages (0 - 255)
    pub opcode: u8,
    // Depends on OpCode. For Reads/Writes, it's the size of the data transported
    // For NAK, it's the number of bytes used for error information (1 or 2)
    pub size: u8,
    // OpCode (of original message) returned in an ACK or NAK response
    pub req_opcode: u8,
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
    pub fn new(
        seq_number: u16,
        session: u8,
        opcode: MavlinkFtpOpcode,
        size: u8,
        req_opcode: MavlinkFtpOpcode,
        burst_complete: u8,
        offset: u32,
        data: Vec<u8>,
    ) -> Self {
        Self {
            seq_number,
            session,
            opcode: opcode as u8,
            size,
            req_opcode: req_opcode as u8,
            burst_complete,
            padding: 0,
            offset,
            data,
        }
    }

    // Convert payload structure into a byte array
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.seq_number.to_le_bytes());
        bytes.push(self.session);
        bytes.push(self.opcode);
        bytes.push(self.size);
        bytes.push(self.req_opcode);
        bytes.push(self.burst_complete);
        bytes.push(self.padding);
        bytes.extend_from_slice(&self.offset.to_le_bytes());
        bytes.extend_from_slice(&self.data);

        bytes
    }
}
