mod cli;
mod data;
mod endpoints;
mod legacy;
mod mavlink_vehicle;
mod server;
mod websocket_manager;

use std::sync::{Arc, Mutex};

use data::MAVLinkMessage;
use legacy::JsonSchema;
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
    let vehicle =
        mavlink_vehicle::MAVLinkVehicleHandle::<mavlink::dialects::ardupilotmega::MavMessage>::new(
            cli::mavlink_connection_string(),
            mavlink_version,
            system_id,
            component_id,
            cli::mavlink_send_initial_heartbeats(),
        );

    let inner_vehicle = vehicle.mavlink_vehicle.clone();
    server::run(cli::server_address(), &inner_vehicle);

    //TODO: Do inside endpoint and use web::Data ?
    websocket_manager::manager()
        .lock()
        .unwrap()
        .new_message_callback = Some(Arc::new(move |value, schema| {
        ws_callback(inner_vehicle.clone(), value, schema)
    }));

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        while let Ok((header, message)) = vehicle.thread_rx_channel.recv() {
            debug!("Received: {:#?} {:#?}", header, message);
            websocket_manager::send(&MAVLinkMessage {
                header,
                message: message.clone(),
            });
            data::update((header, message));
        }
    }
}

fn ws_callback(
    inner_vehicle: Arc<
        Mutex<mavlink_vehicle::MAVLinkVehicle<mavlink::dialects::ardupilotmega::MavMessage>>,
    >,
    value: &str,
    schema: JsonSchema,
) -> String {
    match endpoints::parse_incoming_message(value, schema) {
        Ok(content) => {
            let result = inner_vehicle
                .lock()
                .unwrap()
                .send(&content.header, &content.message);
            match result {
                Ok(_) => {
                    websocket_manager::send(&content);
                    data::update((content.header, content.message));
                    serde_json::json!({ "result": "ok" }).to_string()
                }
                Err(error) => serde_json::json!({ "error": error.to_string() }).to_string(),
            }
        }
        Err(error) => serde_json::json!({ "error": error }).to_string(),
    }
}
