use actix_web::{error::Error, web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};

use super::websocket_manager::WebsocketActor;
use crate::data;

use log::*;

#[derive(Serialize, Debug, Default)]
pub struct InfoContent {
    name: String,
    version: String,
    sha: String,
    build_date: String,
    authors: String,
}

#[derive(Serialize, Debug, Default)]
pub struct Info {
    version: u32,
    service: InfoContent,
}

#[derive(Deserialize)]
pub struct WebsocketQuery {
    filter: Option<String>,
}

#[cfg(debug_assertions)]
fn load_html_file(filename: &str) -> Option<String> {
    let mut filename = filename;
    if filename.is_empty() {
        filename = "index.html";
    }
    let file_path = format!("{}/src/html/{}", env!("CARGO_MANIFEST_DIR"), filename);
    match std::fs::read_to_string(file_path) {
        Ok(content) => Some(content),
        Err(_) => None,
    }
}

#[cfg(not(debug_assertions))]
fn load_html_file(filename: &str) -> Option<String> {
    let index = std::include_str!(concat!("html/", "index.html"));
    let vue = std::include_str!(concat!("html/", "vue.js"));
    match filename {
        "" | "index.html" => Some(index.into()),
        "vue.js" => Some(vue.into()),
        _ => None,
    }
}

pub fn root(req: HttpRequest) -> HttpResponse {
    if let Some(content) = load_html_file(req.match_info().query("filename")) {
        return HttpResponse::Ok().content_type("text/html").body(content);
    };

    return HttpResponse::NotFound()
        .content_type("text/plain")
        .body("File does not exist");
}

pub fn info() -> HttpResponse {
    let info = Info {
        version: 0,
        service: InfoContent {
            name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
            sha: env!("VERGEN_SHA_SHORT").into(),
            build_date: env!("VERGEN_BUILD_DATE").into(),
            authors: env!("CARGO_PKG_AUTHORS").into(),
        },
    };

    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string_pretty(&info).unwrap())
}

pub fn mavlink() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string_pretty(&data::messages()).unwrap())
}

pub async fn websocket(
    req: HttpRequest,
    query: web::Query<WebsocketQuery>,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let filter = match query.into_inner().filter {
        Some(filter) => filter,
        _ => ".*".to_owned(),
    };

    debug!("New websocket with filter {:#?}", &filter);
    let resp = ws::start(WebsocketActor::new(filter), &req, stream);
    resp
}
