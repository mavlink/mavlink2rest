use super::endpoints;
use super::mavlink_vehicle::MAVLinkVehicleArcMutex;

use paperclip::actix::{web, web::Scope, OpenApiExt};

use actix_cors::Cors;
use actix_web::{
    error::{ErrorBadRequest, JsonPayloadError},
    rt::System,
    App, HttpRequest, HttpServer,
};

use crate::cli;
use log::*;

fn json_error_handler(error: JsonPayloadError, _: &HttpRequest) -> actix_web::Error {
    warn!("Problem with json: {}", error.to_string());
    match error {
        JsonPayloadError::Overflow => JsonPayloadError::Overflow.into(),
        _ => ErrorBadRequest(error.to_string()),
    }
}

fn add_v1_paths(scope: Scope) -> Scope {
    scope
        .route("/helper/mavlink", web::get().to(endpoints::helper_mavlink))
        .route("/mavlink", web::get().to(endpoints::mavlink))
        .route("/mavlink", web::post().to(endpoints::mavlink_post))
        .route(r"/mavlink/{path:.*}", web::get().to(endpoints::mavlink))
        .service(web::resource("/ws/mavlink").route(web::get().to(endpoints::websocket)))
}

// Start REST API server with the desired address
pub fn run(server_address: &str, mavlink_vehicle: &MAVLinkVehicleArcMutex) {
    let server_address = server_address.to_string();
    let mavlink_vehicle = mavlink_vehicle.clone();
    println!("Server running: http://{server_address}");

    // Start HTTP server thread
    let _ = System::new("http-server");
    HttpServer::new(move || {
        let v1 = add_v1_paths(web::scope("/v1"));
        let default = match cli::default_api_version() {
            1 => add_v1_paths(web::scope("")),
            _ => unreachable!("CLI should only allow supported values."),
        };
        App::new()
            .wrap(Cors::permissive())
            // Record services and routes for paperclip OpenAPI plugin for Actix.
            .wrap_api()
            //TODO Add middle man to print all http events
            .data(web::JsonConfig::default().error_handler(json_error_handler))
            .data(mavlink_vehicle.clone())
            //TODO: Add cors
            .route("/", web::get().to(endpoints::root))
            .with_json_spec_at("/docs.json")
            .with_swagger_ui_at("/docs")
            .route(
                r"/{filename:.*(\.html|\.js|\.css)}",
                web::get().to(endpoints::root),
            )
            .route("/info", web::get().to(endpoints::info))
            .service(default)
            .service(v1)
            .build()
    })
    .bind(server_address)
    .unwrap()
    .run();
}
