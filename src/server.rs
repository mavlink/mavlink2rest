use super::endpoints;
use super::mavlink_vehicle::MAVLinkVehicleArcMutex;

use paperclip::actix::{web::get, OpenApiExt};
use std::sync::{Arc, Mutex};

use actix_web::{
    error::{ErrorBadRequest, JsonPayloadError},
    rt::System,
    web, App, HttpRequest, HttpServer,
};

use log::*;

fn json_error_handler(error: JsonPayloadError, _: &HttpRequest) -> actix_web::Error {
    warn!("Problem with json: {}", error.to_string());
    match error {
        JsonPayloadError::Overflow => JsonPayloadError::Overflow.into(),
        _ => ErrorBadRequest(error.to_string()),
    }
}

// Start REST API server with the desired address
pub fn run(server_address: &str, mavlink_vehicle: &MAVLinkVehicleArcMutex) {
    let server_address = server_address.to_string();
    let mavlink_vehicle = mavlink_vehicle.clone();

    // Start HTTP server thread
    let _ = System::new("http-server");
    HttpServer::new(move || {
        App::new()
            // Record services and routes for paperclip OpenAPI plugin for Actix.
            .wrap_api()
            //TODO Add middle man to print all http events
            .data(web::JsonConfig::default().error_handler(json_error_handler))
            .data(mavlink_vehicle.clone())
            //TODO: Add cors
            .route("/", web::get().to(endpoints::root))
            .route(
                r"/{filename:.*(\.html|\.js)}",
                web::get().to(endpoints::root),
            )
            .route("/helper/mavlink", web::get().to(endpoints::helper_mavlink))
            .route("/info", web::get().to(endpoints::info))
            .route("/mavlink", web::get().to(endpoints::mavlink))
            .route("/mavlink", web::post().to(endpoints::mavlink_post))
            .route(r"/mavlink/{path:.*}", web::get().to(endpoints::mavlink))
            .service(web::resource("/ws/mavlink").route(web::get().to(endpoints::websocket)))
            .with_json_spec_at("/api-docs")
    })
    .bind(server_address)
    .unwrap()
    .run();
}
