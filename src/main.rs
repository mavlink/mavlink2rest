use std::sync::{Arc, Mutex};

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
            clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Be verbose")
                .takes_value(false),
        )
        .get_matches();

    let verbose = matches.is_present("verbose");
    let server_string = matches.value_of("server").unwrap();
    let connection_string = matches.value_of("connect").unwrap();

    let mut vehicle = Vehicle::new(connection_string, verbose);
    vehicle.run();

    let inner_vehicle = Arc::clone(&vehicle.inner);
    let inner_vehicle = inner_vehicle.lock().unwrap();
    let api = Arc::new(Mutex::new(API::new(Arc::clone(&inner_vehicle.messages))));

    println!("MAVLink connection string: {}", connection_string);
    println!("REST API address: {}", server_string);

    HttpServer::new(move || {
        let cloned_api_root = api.clone();
        let cloned_api_get_mavlink = api.clone();
        App::new()
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
    })
    .bind(server_string)
    .unwrap()
    .run()
    .unwrap();
}
