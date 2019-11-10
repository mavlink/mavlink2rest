use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use clap;
use serde_json::json;

mod message_information;
use message_information::MessageInformation;

use lazy_static::lazy_static;
lazy_static! {
    static ref MESSAGES: std::sync::Arc<Mutex<serde_json::value::Value>> = {
        // Create an empty map with the main key as mavlink
        return Arc::new(Mutex::new(json!({"mavlink":{}})));
    };
}

fn main() {
    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("MAVLink to REST API!.")
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            clap::Arg::with_name("connect")
                .short("c")
                .long("connect")
                .value_name("TYPE:<IP/SERIAL>:<PORT/BAUDRATE>")
                .help("Sets the mavlink connection string")
                .takes_value(true)
                .default_value("udpin:0.0.0.0:14550"),
        )
        .arg(
            clap::Arg::with_name("server")
                .short("s")
                .long("server")
                .value_name("IP:PORT")
                .help("Sets the IP and port that the rest server will be provided")
                .takes_value(true)
                .default_value("0.0.0.0:8088"),
        )
        .get_matches();

    let server_string = matches.value_of("server").unwrap();
    let connection_string = matches.value_of("connect").unwrap();

    println!("MAVLink connection string: {}", connection_string);
    println!("REST API address: {}", server_string);

    let mavconn = mavlink::connect(connection_string).unwrap();

    let vehicle = Arc::new(mavconn);
    let _ = vehicle.send_default(&request_stream());

    thread::spawn({
        let vehicle = vehicle.clone();
        move || loop {
            let res = vehicle.send_default(&heartbeat_message());
            if res.is_ok() {
                thread::sleep(Duration::from_secs(1));
            } else {
                println!("Failed to send heartbeat");
            }
        }
    });

    thread::spawn({
        let vehicle = vehicle.clone();
        let messages_ref = Arc::clone(&MESSAGES);

        let mut messages_information: std::collections::HashMap<
            std::string::String,
            MessageInformation,
        > = std::collections::HashMap::new();
        move || {
            loop {
                match vehicle.recv() {
                    Ok((_header, msg)) => {
                        let value = serde_json::to_value(&msg).unwrap();
                        let mut msgs = messages_ref.lock().unwrap();
                        // Remove " from string
                        let msg_type = value["type"].to_string().replace("\"", "");
                        msgs["mavlink"][&msg_type] = value;

                        // Update message_information
                        let message_information = messages_information
                            .entry(msg_type.clone())
                            .or_insert(MessageInformation::default());
                        message_information.update();
                        msgs["mavlink"][&msg_type]["message_information"] =
                            serde_json::to_value(messages_information[&msg_type]).unwrap();
                    }
                    Err(e) => {
                        match e.kind() {
                            std::io::ErrorKind::WouldBlock => {
                                //no messages currently available to receive -- wait a while
                                thread::sleep(Duration::from_secs(1));
                                continue;
                            }
                            _ => {
                                println!("recv error: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            }
        }
    });

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(root_page))
            .route("/mavlink|/mavlink/*", web::get().to(mavlink_page))
    })
    .bind(server_string)
    .unwrap()
    .run()
    .unwrap();
}

fn root_page(_req: HttpRequest) -> impl Responder {
    return "Wubba Lubba dub-dub".to_string();
}

fn mavlink_page(req: HttpRequest) -> impl Responder {
    let url_path = req.uri().to_string();
    let messages_ref = Arc::clone(&MESSAGES);
    let message = messages_ref.lock().unwrap();
    let final_result = (*message).pointer(&url_path);

    if final_result.is_none() {
        return "No valid path".to_string();
    }
    return serde_json::to_string(final_result.unwrap())
        .unwrap()
        .to_string();
}

pub fn heartbeat_message() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::common::MavType::MAV_TYPE_QUADROTOR,
        autopilot: mavlink::common::MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
        base_mode: mavlink::common::MavModeFlag::empty(),
        system_status: mavlink::common::MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    })
}

pub fn request_stream() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::REQUEST_DATA_STREAM(mavlink::common::REQUEST_DATA_STREAM_DATA {
        target_system: 0,
        target_component: 0,
        req_stream_id: 0,
        req_message_rate: 10,
        start_stop: 1,
    })
}
