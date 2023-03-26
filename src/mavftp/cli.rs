use structopt::StructOpt;
use std::path::PathBuf;

#[derive(Debug, StructOpt)]
#[structopt(name = "mavlink_ftp_cli", about = "Mavlink FTP CLI")]
pub struct Opt {
    #[structopt(long = "connection", default_value = "tcpout:0.0.0.0:5760", help = "Connection string")]
    pub connection: String,

    #[structopt(subcommand)]
    pub cmd: MavlinkFTPCommand,
}

#[derive(Debug, StructOpt)]
pub enum MavlinkFTPCommand {
    /// List files in a directory
    #[structopt(name = "list")]
    ListDirectory {
        /// Directory path
        #[structopt(parse(from_os_str), default_value = ".")]
        path: PathBuf,
    },
    /// Read a file
    #[structopt(name = "read")]
    ReadFile {
        /// File path
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Create a file
    #[structopt(name = "create")]
    CreateFile {
        /// File path
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Write to a file
    #[structopt(name = "write")]
    WriteFile {
        /// File path
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Remove a file
    #[structopt(name = "remove")]
    RemoveFile {
        /// File path
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Create a directory
    #[structopt(name = "mkdir")]
    CreateDirectory {
        /// Directory path
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Remove a directory
    #[structopt(name = "rmdir")]
    RemoveDirectory {
        /// Directory path
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Calculate CRC32 for a file
    #[structopt(name = "crc")]
    CalcFileCRC32 {
        /// File path
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
}