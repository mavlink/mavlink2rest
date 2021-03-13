#[macro_use]
extern crate lazy_static;
extern crate env_logger;

mod cli;
mod data;
mod endpoints;
mod mavlink_vehicle;
mod server;

use log::*;

fn main() -> std::io::Result<()> {
    env_logger::init();
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

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        while let Ok((header, message)) = vehicle.thread_rx_channel.recv() {
            debug!("Received: {:#?} {:#?}", header, message);
            data::update((header, message));
        }
    }
}
