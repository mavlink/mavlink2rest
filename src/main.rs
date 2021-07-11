use std::sync::{Arc, Mutex};

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use clap;

mod message_information;

mod vehicle_handler;
use vehicle_handler::Vehicle;

mod rest_api;
use rest_api::API;

fn main() {
    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("MAVLink to REST API!")
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
        .arg(
            clap::Arg::with_name("mavlink")
                .long("mavlink")
                .value_name("VERSION")
                .help("Sets the mavlink version used to communicate")
                .takes_value(true)
                .default_value("2"),
        )
        .arg(
            clap::Arg::with_name("system_id")
                .long("system-id")
                .value_name("SYSTEM_ID")
                .help("Sets system ID for this service.")
                .takes_value(true)
                .default_value("255"),
        )
        .arg(
            clap::Arg::with_name("component_id")
                .long("component-id")
                .value_name("COMPONENT_ID")
                .help("Sets the component ID for this service, for more information, check: https://mavlink.io/en/messages/common.html#MAV_COMPONENT")
                .takes_value(true)
                .default_value("0"),
        )
        .arg(
            clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Be verbose")
                .takes_value(false),
        )
        .get_matches();

    let verbose = matches.is_present("verbose");
    let mavlink_version = matches.value_of("mavlink").unwrap();
    let server_string = matches.value_of("server").unwrap();
    let connection_string = matches.value_of("connect").unwrap();

    let system_id = matches
        .value_of("system_id")
        .unwrap()
        .parse::<u8>()
        .expect("System ID should be a value between 1-255.");
    let component_id = matches
        .value_of("component_id")
        .unwrap()
        .parse::<u8>()
        .expect("Component ID should be a value between 1-255.");

    let mavlink_version = match mavlink_version {
        "1" => mavlink::MavlinkVersion::V1,
        "2" => mavlink::MavlinkVersion::V2,
        _ => panic!("Invalid mavlink version"),
    };

    let mut vehicle = Vehicle::new(connection_string, mavlink_version, verbose);
    vehicle.set_system_id(system_id);
    vehicle.set_component_id(component_id);
    vehicle.run();

    let inner_vehicle = Arc::clone(&vehicle.inner);
    let inner_vehicle_message = inner_vehicle.lock().unwrap();
    let api = Arc::new(Mutex::new(API::new(Arc::clone(
        &inner_vehicle_message.messages,
    ))));

    println!("MAVLink connection string: {}", connection_string);
    println!("REST API address: {}", server_string);

    // Remove guard after clone
    std::mem::drop(inner_vehicle_message);
    let inner_vehicle = Arc::clone(&vehicle.inner);

    HttpServer::new(move || {
        let inner_vehicle = inner_vehicle.clone();
        let cloned_api_root = api.clone();
        let cloned_api_get_mavlink = api.clone();
        let cloned_api_post_mavlink = api.clone();
        let cloned_api_helper_page = api.clone();
        App::new()
            .wrap(Cors::default())
            .route(
                "/",
                web::get().to(move || {
                    let api = cloned_api_root.lock().unwrap();
                    api.root_page()
                }),
            )
            .route(
                "/mavlink|/mavlink/*",
                web::get().to(move |x| {
                    let api = cloned_api_get_mavlink.lock().unwrap();
                    api.mavlink_page(x)
                }),
            )
            .route(
                "/helper/message/*",
                web::get().to(move |x| {
                    let api = cloned_api_helper_page.lock().unwrap();
                    api.mavlink_helper_page(x)
                }),
            )
            .route(
                "/mavlink",
                web::post().to(move |x| {
                    let inner_vehicle = inner_vehicle.lock().unwrap();
                    let mut api = cloned_api_post_mavlink.lock().unwrap();
                    let content = api.mavlink_post(x);
                    inner_vehicle
                        .channel
                        .send(&content.header, &content.message)
                }),
            )
    })
    .bind(server_string)
    .unwrap()
    .run()
    .unwrap();
}
