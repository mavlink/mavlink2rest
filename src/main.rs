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

    let (system_id, component_id) = cli::mavlink_system_and_component_id();
    let vehicle = mavlink_vehicle::MAVLinkVehicleHandle::<mavlink::ardupilotmega::MavMessage>::new(
        cli::mavlink_connection_string(),
        mavlink_version,
        system_id,
        component_id,
    );

    let inner_vehicle = vehicle.mavlink_vehicle.clone();
    server::run(cli::server_address(), &inner_vehicle.clone());

    //TODO: Do inside endpoint and use web::Data ?
    websocket_manager::manager()
        .lock()
        .unwrap()
        .new_message_callback = Some(std::sync::Arc::new(move |value: &String| {
        if let Ok(content @ MAVLinkMessage::<mavlink::ardupilotmega::MavMessage> { .. }) =
            serde_json::from_str(value)
        {
            let result = inner_vehicle
                .lock()
                .unwrap()
                .send(&content.header, &content.message);
            return format!("{:?}", result);
        } else if let Ok(content @ MAVLinkMessage::<mavlink::common::MavMessage> { .. }) =
            serde_json::from_str(value)
        {
            let result = inner_vehicle.lock().unwrap().send(
                &content.header,
                &mavlink::ardupilotmega::MavMessage::common(content.message),
            );
            return format!("{:?}", result);
        };

        return "Could not convert input message.".into();
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
