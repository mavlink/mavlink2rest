use std::sync::{Arc, Mutex};

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use clap;

mod message_information;

mod vehicle_handler;
use vehicle_handler::{InnerVehicle, Vehicle};

mod rest_api;
use rest_api::API;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
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

    let mavlink_version = match mavlink_version {
        "1" => mavlink::MavlinkVersion::V1,
        "2" => mavlink::MavlinkVersion::V2,
        _ => panic!("Invalid mavlink version"),
    };

    let mut vehicle = Vehicle::new(connection_string, mavlink_version, verbose);
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
        let api = api.clone();
        struct RestData {
            api: Arc<Mutex<API>>,
            vehicle: Arc<Mutex<InnerVehicle>>,
        };

        let data = RestData {
            api: api.clone(),
            vehicle: inner_vehicle.clone(),
        };
        App::new()
            .wrap(Cors::default())
            .wrap(middleware::NormalizePath)
            .data(Arc::new(Mutex::new(data)))
            .service(web::resource("/").route(web::get().to(
                |data: web::Data<Arc<Mutex<RestData>>>| async move {
                    let answer = data.lock().unwrap().api.lock().unwrap().root_page();
                    return answer;
                },
            )))
            .service(
                // Needs https://github.com/actix/actix-web/pull/1639 to accept /mavlink/
                web::scope("/mavlink")
                    .route(
                        "*",
                        web::get().to(|data: web::Data<Arc<Mutex<RestData>>>, bytes| async move {
                            let answer =
                                data.lock().unwrap().api.lock().unwrap().mavlink_page(bytes);
                            return answer;
                        }),
                    )
                    .route(
                        "",
                        web::get().to(|data: web::Data<Arc<Mutex<RestData>>>, bytes| async move {
                            let answer =
                                data.lock().unwrap().api.lock().unwrap().mavlink_page(bytes);
                            return answer;
                        }),
                    )
                    .route(
                        "",
                        web::post().to(|data: web::Data<Arc<Mutex<RestData>>>, bytes| async move {
                            let content =
                                data.lock().unwrap().api.lock().unwrap().mavlink_post(bytes);
                            if content.is_err() {
                                return HttpResponse::NotFound().content_type("text/plain").body(
                                    format!(
                                        "Error: {}",
                                        content.err().unwrap().into_inner().unwrap()
                                    ),
                                );
                            }
                            let msg = content.unwrap();
                            let result = data
                                .lock()
                                .unwrap()
                                .vehicle
                                .lock()
                                .unwrap()
                                .channel
                                .send(&msg.header, &msg.message);
                            if result.is_err() {
                                return HttpResponse::NotFound().content_type("text/plain").body(
                                    format!(
                                        "Error: {:#?}",
                                        result.err().unwrap().into_inner().unwrap()
                                    ),
                                );
                            }

                            return HttpResponse::Ok()
                                .content_type("text/plain")
                                .body(format!("{:#?}", result.ok().unwrap()));
                        }),
                    ),
            )
            .service(web::resource("/helper/message/*").route(web::get().to(
                |data: web::Data<Arc<Mutex<RestData>>>, bytes| async move {
                    let answer = data
                        .lock()
                        .unwrap()
                        .api
                        .lock()
                        .unwrap()
                        .mavlink_helper_page(bytes);
                    return answer;
                },
            )))
    })
    .bind(server_string)
    .unwrap()
    .run()
    .await
}
