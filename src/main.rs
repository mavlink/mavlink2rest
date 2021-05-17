#[macro_use]
extern crate lazy_static;
extern crate env_logger;

mod cli;
mod data;
mod endpoints;
mod mavlink_vehicle;
mod server;
mod websocket_manager;

use data::MAVLinkMessage;
use endpoints::MAVLinkMessageCommon;
use log::*;

fn main() -> std::io::Result<()> {
    let log_filter = if cli::is_verbose() { "debug" } else { "warn" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_filter)).init();
    cli::init();

    let mavlink_version = match cli::mavlink_version() {
        1 => mavlink::MavlinkVersion::V1,
        2 => mavlink::MavlinkVersion::V2,
        _ => panic!("Invalid mavlink version."),
    };

    let vehicle = mavlink_vehicle::MAVLinkVehicleHandle::<mavlink::ardupilotmega::MavMessage>::new(
        cli::mavlink_connection_string(),
        mavlink_version,
    );

    server::run(cli::server_address());

    websocket_manager::manager()
        .lock()
        .unwrap()
        .new_message_callback = Some(std::sync::Arc::new(move |value: &String| {
        if let Ok(content @ MAVLinkMessage { .. }) = serde_json::from_str(value) {
            dbg!("ardupilotmega", content);
        }
        if let Ok(content @ MAVLinkMessageCommon { .. }) = serde_json::from_str(value) {
            dbg!("common", content);
        }
        "Ok".into()
    }));

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        while let Ok((header, message)) = vehicle.thread_rx_channel.recv() {
            debug!("Received: {:#?} {:#?}", header, message);
            websocket_manager::send(&message);
            data::update((header, message));
        }
    }
}
