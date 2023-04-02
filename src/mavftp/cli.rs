use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "mavlink_ftp_cli", about = "Mavlink FTP CLI")]
pub struct Opt {
    #[structopt(
        long = "connection",
        default_value = "tcpout:0.0.0.0:5760",
        help = "Connection string"
    )]
    pub connection: String,

    #[structopt(subcommand)]
    pub command: MavlinkFTPCommand,
}

#[derive(Debug, StructOpt)]
pub enum MavlinkFTPCommand {
    /// List files in a directory
    #[structopt(name = "list")]
    ListDirectory {
        /// Directory path
        #[structopt(default_value = ".")]
        path: String,
    },
    /// Read a file
    #[structopt(name = "read")]
    ReadFile {
        /// File path
        path: String,
    },
    /// Create a file
    #[structopt(name = "create")]
    CreateFile {
        /// File path
        path: String,
    },
    /// Write to a file
    #[structopt(name = "write")]
    WriteFile {
        /// File path
        path: String,
    },
    /// Remove a file
    #[structopt(name = "remove")]
    RemoveFile {
        /// File path
        path: String,
    },
    /// Create a directory
    #[structopt(name = "mkdir")]
    CreateDirectory {
        /// Directory path
        path: String,
    },
    /// Remove a directory
    #[structopt(name = "rmdir")]
    RemoveDirectory {
        /// Directory path
        path: String,
    },
    /// Calculate CRC32 for a file
    #[structopt(name = "crc")]
    CalcFileCRC32 {
        /// File path
        path: String,
    },
    /// Reset sessions
    #[structopt(name = "reset")]
    Reset,
}
