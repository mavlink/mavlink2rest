use super::endpoints;

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
pub fn run(server_address: &str) {
    let server_address = server_address.to_string();

    // Start HTTP server thread
    let _ = System::new("http-server");
    HttpServer::new(|| {
        App::new()
            //TODO Add middle man to print all http events
            .data(web::JsonConfig::default().error_handler(json_error_handler))
            //TODO: Add cors
            .route("/", web::get().to(endpoints::root))
            .route(
                r"/{filename:.*(\.html|\.js)}",
                web::get().to(endpoints::root),
            )
            .route("/info", web::get().to(endpoints::info))
            .route("/mavlink", web::get().to(endpoints::mavlink))
    })
    .bind(server_address)
    .unwrap()
    .run();
}
