//! Records received MAVLink messages to a `.tlog` file (the format
//! QGroundControl, Mission Planner and MAVProxy read: each frame behind an
//! 8 byte big-endian timestamp in microseconds since the Unix epoch).

use std::fs::File;
use std::io::BufWriter;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lazy_static::lazy_static;
use log::*;
use mavlink_tlog::TlogWriter;

use crate::cli;

struct Recorder {
    writer: TlogWriter<BufWriter<File>>,
    version: mavlink::MavlinkVersion,
    last_flush: Instant,
    buffer: Vec<u8>,
}

lazy_static! {
    static ref RECORDER: Mutex<Option<Recorder>> = Mutex::new(None);
}

const FLUSH_INTERVAL: Duration = Duration::from_secs(1);

/// Opens the file given with `--tlog`, if any. Messages are written with the
/// MAVLink version the service talks to the vehicle.
pub fn init(version: mavlink::MavlinkVersion) -> std::io::Result<()> {
    let path = match cli::tlog_path() {
        Some(path) => path,
        None => return Ok(()),
    };
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    *RECORDER.lock().unwrap() = Some(Recorder {
        writer: TlogWriter::new(BufWriter::new(file)),
        version,
        last_flush: Instant::now(),
        buffer: Vec::with_capacity(280),
    });
    info!("Recording MAVLink messages to {}", path);
    Ok(())
}

/// Appends one received message to the log, if recording is enabled.
pub fn record<M: mavlink::Message>(header: &mavlink::MavHeader, message: &M) {
    let mut guard = RECORDER.lock().unwrap();
    let recorder = match guard.as_mut() {
        Some(recorder) => recorder,
        None => return,
    };
    recorder.buffer.clear();
    if let Err(error) =
        mavlink::write_versioned_msg(&mut recorder.buffer, recorder.version, *header, message)
    {
        warn!("Failed to serialize message for tlog: {:?}", error);
        return;
    }
    if let Err(error) = recorder.writer.write_frame_now(&recorder.buffer) {
        warn!("Failed to write tlog: {:?}", error);
        return;
    }
    if recorder.last_flush.elapsed() >= FLUSH_INTERVAL {
        if let Err(error) = recorder.writer.flush() {
            warn!("Failed to flush tlog: {:?}", error);
        }
        recorder.last_flush = Instant::now();
    }
}

/// Flushes the log and returns its path, or `None` when not recording.
pub fn flush() -> Option<&'static str> {
    let mut guard = RECORDER.lock().unwrap();
    let recorder = guard.as_mut()?;
    if let Err(error) = recorder.writer.flush() {
        warn!("Failed to flush tlog: {:?}", error);
    }
    recorder.last_flush = Instant::now();
    cli::tlog_path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::dialects::ardupilotmega::MavMessage;

    #[test]
    fn recorded_messages_read_back() {
        let path =
            std::env::temp_dir().join(format!("mavlink2rest-tlog-{}.tlog", std::process::id()));
        let file = File::create(&path).unwrap();
        let mut recorder = Recorder {
            writer: TlogWriter::new(BufWriter::new(file)),
            version: mavlink::MavlinkVersion::V2,
            last_flush: Instant::now(),
            buffer: Vec::new(),
        };
        let header = mavlink::MavHeader {
            system_id: 1,
            component_id: 1,
            sequence: 7,
        };
        let message = crate::mavlink_vehicle::heartbeat_message();
        for _ in 0..3 {
            recorder.buffer.clear();
            mavlink::write_versioned_msg(&mut recorder.buffer, recorder.version, header, &message)
                .unwrap();
            recorder.writer.write_frame_now(&recorder.buffer).unwrap();
        }
        recorder.writer.flush().unwrap();
        drop(recorder);

        let bytes = std::fs::read(&path).unwrap();
        let records: Vec<_> = mavlink_tlog::TlogSlice::with_options(
            &bytes,
            mavlink_tlog::ReadOptions {
                crc: mavlink_tlog::CrcPolicy::Strict,
                ..Default::default()
            },
        )
        .collect();
        assert_eq!(records.len(), 3);
        for record in &records {
            assert_eq!(record.frame.message_name(), Some("HEARTBEAT"));
            assert_eq!((record.frame.sys_id(), record.frame.seq()), (1, 7));
            // The frame decodes back to the message that was recorded.
            let mut reader = mavlink::MavlinkReader::new(record.frame.as_bytes());
            let (decoded_header, decoded) = reader.read_any_message::<MavMessage>().unwrap();
            assert_eq!(decoded_header, header);
            assert_eq!(decoded, message);
        }
        std::fs::remove_file(&path).ok();
    }
}
