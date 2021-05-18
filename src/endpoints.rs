use actix_web::{error::Error, web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};

use super::websocket_manager::WebsocketActor;
use crate::data;

use log::*;
use mavlink::Message;

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

#[derive(Deserialize)]
pub struct MAVLinkHelperQuery {
    name: String,
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

pub fn mavlink(req: HttpRequest) -> HttpResponse {
    let path = req.match_info().query("path");
    HttpResponse::Ok()
        .content_type("application/json")
        .body(data::messages().pointer(path))
}

pub fn parse_query<T: serde::ser::Serialize>(message: &T) -> String {
    let error_message =
        "Not possible to parse mavlink message, please report this issue!".to_string();
    serde_json::to_string_pretty(&message).unwrap_or(error_message)
}

pub fn helper_mavlink(_req: HttpRequest, query: web::Query<MAVLinkHelperQuery>) -> HttpResponse {
    let message_name = query.into_inner().name;

    let result = match mavlink::ardupilotmega::MavMessage::message_id_from_name(&message_name) {
        Ok(id) => mavlink::Message::default_message_from_id(id),
        Err(error) => Err(error),
    };

    match result {
        Ok(result) => {
            match result {
                mavlink::ardupilotmega::MavMessage::common(msg) => {
                    let result = data::MAVLinkMessage {
                        header: mavlink::MavHeader::default(),
                        message: msg,
                    };

                    return HttpResponse::Ok()
                        .content_type("application/json")
                        .body(parse_query(&result));
                }
                msg => {
                    let result = data::MAVLinkMessage {
                        header: mavlink::MavHeader::default(),
                        message: msg,
                    };

                    return HttpResponse::Ok()
                        .content_type("application/json")
                        .body(parse_query(&result));
                }
            };
        }
        Err(content) => {
            return HttpResponse::NotFound()
                .content_type("application/json")
                .body(parse_query(&content));
        }
    }
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
