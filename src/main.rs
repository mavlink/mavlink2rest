#[macro_use]
extern crate lazy_static;

use std::sync::{Arc, Mutex};

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use serde_derive::Deserialize;

mod message_information;

mod vehicle_handler;
use vehicle_handler::{InnerVehicle, Vehicle};

mod rest_api;
use rest_api::API;

mod websocket_manager;
use websocket_manager::{WebsocketActor, WebsocketError, WebsocketManager};

mod cli;
mod mavlink_vehicle;

#[derive(Deserialize)]
struct WebsocketQuery {
    filter: Option<String>,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    cli::init();
    mavlink_vehicle::MAVLinkVehicleHandle::new("udpin:0.0.0.0:3333");

    let mavlink_version = match cli::mavlink_version() {
        1 => mavlink::MavlinkVersion::V1,
        2 => mavlink::MavlinkVersion::V2,
        _ => panic!("Invalid mavlink version"),
    };

    let mut vehicle = Vehicle::new(
        cli::mavlink_connection_string(),
        mavlink_version,
        cli::is_verbose(),
    );

    let websocket = Arc::new(Mutex::new(WebsocketManager::default()));

    let callback_webscoket = websocket.clone();
    vehicle.inner.lock().unwrap().new_message_callback = Some(Arc::new(move |value, name| {
        callback_webscoket.lock().unwrap().send(value, name);
    }));

    vehicle.run();

    let inner_vehicle = Arc::clone(&vehicle.inner);
    let inner_vehicle_message = inner_vehicle.lock().unwrap();
    let api = Arc::new(Mutex::new(API::new(Arc::clone(
        &inner_vehicle_message.messages,
    ))));

    let callback_inner_vehicle = inner_vehicle.clone();
    let callback_api = api.clone();
    let callback_webscoket = websocket.clone();
    callback_webscoket.lock().unwrap().new_message_callback = Some(Arc::new(move |value| {
        let content = callback_api
            .lock()
            .unwrap()
            .extract_mavlink_from_string(value);
        if content.is_err() {
            return serde_json::to_string(&WebsocketError {
                error: format!("{:#?}", content),
            })
            .unwrap();
        }
        let msg = content.unwrap();
        let result = callback_inner_vehicle
            .lock()
            .unwrap()
            .channel
            .send(&msg.header, &msg.message);

        format!("{:#?}", result)
    }));

    println!(
        "MAVLink connection string: {}",
        cli::mavlink_connection_string()
    );
    println!("REST API address: {}", cli::server_address());

    // Remove guard after clone
    std::mem::drop(inner_vehicle_message);
    let inner_vehicle = Arc::clone(&vehicle.inner);

    HttpServer::new(move || {
        let api = api.clone();
        struct RestData {
            api: Arc<Mutex<API>>, // Move to actix ADDR
            vehicle: Arc<Mutex<InnerVehicle>>,
            websocket: Arc<Mutex<WebsocketManager>>,
        };

        let data = RestData {
            api,
            vehicle: inner_vehicle.clone(),
            websocket: websocket.clone(),
        };
        App::new()
            .wrap(Cors::default())
            .wrap(middleware::NormalizePath)
            .data(Arc::new(Mutex::new(data)))
            .service(web::resource("/").route(web::get().to(rest_api::redirect_root_page)))
            .service(
                web::resource(r"/{filename:.*(\.html|\.js)}")
                    .route(web::get().to(rest_api::root_page)),
            )
            .service(web::resource("/info").route(web::get().to(rest_api::info)))
            .service(web::resource("/ws/mavlink").route(web::get().to(
                |data: web::Data<Arc<Mutex<RestData>>>,
                 req: HttpRequest,
                 query: web::Query<WebsocketQuery>,
                 stream: web::Payload| async move {
                    let filter = match query.into_inner().filter {
                        Some(filter) => filter,
                        _ => ".*".to_owned(),
                    };
                    let server = data.lock().unwrap().websocket.clone();
                    println!("New websocket with filter {:#?}", &filter);
                    let resp = ws::start(WebsocketActor::new(filter, server), &req, stream);
                    resp
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
                            answer
                        }),
                    )
                    .route(
                        "",
                        web::get().to(|data: web::Data<Arc<Mutex<RestData>>>, bytes| async move {
                            let answer =
                                data.lock().unwrap().api.lock().unwrap().mavlink_page(bytes);
                            answer
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
                    answer
                },
            )))
    })
    .bind(cli::server_address())
    .unwrap()
    .run()
    .await
}
