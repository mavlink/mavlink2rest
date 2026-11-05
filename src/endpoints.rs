use std::path::Path;

use actix_web::{
    web::{self, Json},
    HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use include_dir::{include_dir, Dir};
use paperclip::actix::{api_v2_operation, Apiv2Schema};
use serde::{Deserialize, Serialize};

use super::data;
use super::legacy;
use super::legacy::JsonSchema;
use super::mavlink_vehicle::MAVLinkVehicleArcMutex;
use super::websocket_manager::{self, WebsocketActor};

use log::*;
use mavlink::Message;

static HTML_DIST: Dir<'_> = include_dir!("src/html");

#[derive(Apiv2Schema, Serialize, Debug, Default)]
pub struct InfoContent {
    /// Name of the program
    name: String,
    /// Version/tag
    version: String,
    /// Git SHA
    sha: String,
    build_date: String,
    /// Authors name
    authors: String,
}

#[derive(Apiv2Schema, Serialize, Debug, Default)]
pub struct Info {
    /// Version of the REST API
    version: u32,
    /// Service information
    service: InfoContent,
}

#[derive(Apiv2Schema, Deserialize)]
pub struct WebsocketQuery {
    /// Regex filter to selected the desired MAVLink messages by name
    filter: Option<String>,
    /// Only messages from this system id
    system_id: Option<u8>,
    /// Only messages from this component id
    component_id: Option<u8>,
}

#[derive(Apiv2Schema, Deserialize)]
pub struct MAVLinkHelperQuery {
    /// MAVLink message name, possible options are here: https://docs.rs/mavlink/0.10.0/mavlink/#modules
    name: String,
}

fn load_html_file(filename: &str) -> Option<String> {
    if let Some(file) = HTML_DIST.get_file(filename) {
        return Some(file.contents_utf8().unwrap().to_string());
    }

    None
}

pub fn root(req: HttpRequest) -> HttpResponse {
    let mut filename = req.match_info().query("filename");
    if filename.is_empty() {
        filename = "index.html";
    }

    if let Some(content) = load_html_file(filename) {
        let extension = Path::new(&filename)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");
        let mime = actix_files::file_extension_to_mime(extension).to_string();

        return HttpResponse::Ok().content_type(mime).body(content);
    };

    return HttpResponse::NotFound()
        .content_type("text/plain")
        .body("File does not exist");
}

#[api_v2_operation]
/// Provides information about the API and this program
pub async fn info() -> Json<Info> {
    let info = Info {
        version: 0,
        service: InfoContent {
            name: env!("CARGO_PKG_NAME").into(),
            version: env!("VERGEN_GIT_SEMVER").into(),
            sha: env!("VERGEN_GIT_SHA").into(),
            build_date: env!("VERGEN_BUILD_TIMESTAMP").into(),
            authors: env!("CARGO_PKG_AUTHORS").into(),
        },
    };

    Json(info)
}

#[api_v2_operation]
/// Provide instructions on how to use this API
pub async fn llm() -> actix_web::Result<HttpResponse> {
    let content = load_html_file("llm").unwrap();
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(content)
        .await
}

#[api_v2_operation]
/// Provides an object containing all MAVLink messages received by the service
pub async fn mavlink(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    mavlink_with_schema(req, JsonSchema::Modern).await
}

#[api_v2_operation]
/// Provides MAVLink messages using the legacy 0.x JSON schema
pub async fn mavlink_legacy(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    mavlink_with_schema(req, JsonSchema::Legacy).await
}

async fn mavlink_with_schema(
    req: HttpRequest,
    schema: JsonSchema,
) -> actix_web::Result<HttpResponse> {
    let path = req.match_info().query("path");
    match data::messages().pointer_with_schema(path, schema) {
        Ok(message) => ok_response(message).await,
        Err(error) => not_found_response(error).await,
    }
}

pub fn parse_query<T: serde::ser::Serialize>(message: &T) -> String {
    let error_message =
        "Not possible to parse mavlink message, please report this issue!".to_string();
    serde_json::to_string_pretty(&message).unwrap_or(error_message)
}

fn parse_query_schema<T: serde::ser::Serialize>(
    message: &T,
    schema: JsonSchema,
) -> Result<String, String> {
    let value = serde_json::to_value(message).map_err(|error| error.to_string())?;
    let value = match schema {
        JsonSchema::Modern => value,
        JsonSchema::Legacy => legacy::modern_to_legacy(&value)?,
    };
    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
}

#[api_v2_operation]
/// Returns an example of a MAVLink message matching the given message name
pub async fn helper_mavlink(
    _req: HttpRequest,
    query: web::Query<MAVLinkHelperQuery>,
) -> actix_web::Result<HttpResponse> {
    helper_mavlink_with_schema(query, JsonSchema::Modern).await
}

#[api_v2_operation]
/// Returns a legacy 0.x JSON example for the given MAVLink message name
pub async fn helper_mavlink_legacy(
    _req: HttpRequest,
    query: web::Query<MAVLinkHelperQuery>,
) -> actix_web::Result<HttpResponse> {
    helper_mavlink_with_schema(query, JsonSchema::Legacy).await
}

async fn helper_mavlink_with_schema(
    query: web::Query<MAVLinkHelperQuery>,
    schema: JsonSchema,
) -> actix_web::Result<HttpResponse> {
    let message_name = query.into_inner().name;

    let result =
        match mavlink::dialects::ardupilotmega::MavMessage::message_id_from_name(&message_name) {
            Some(id) => mavlink::dialects::ardupilotmega::MavMessage::default_message_from_id(id)
                .ok_or("Invalid message name."),
            None => Err("Invalid message name."),
        };

    match result {
        Ok(msg) => {
            let envelope = data::MAVLinkMessage {
                header: mavlink::MavHeader::default(),
                message: msg,
            };
            match parse_query_schema(&envelope, schema) {
                Ok(content) => ok_response(content).await,
                Err(error) => not_found_response(error).await,
            }
        }
        Err(content) => not_found_response(parse_query(&content)).await,
    }
}

#[api_v2_operation]
#[allow(clippy::await_holding_lock)]
/// Send a MAVLink message for the desired vehicle
pub async fn mavlink_post(
    data: web::Data<MAVLinkVehicleArcMutex>,
    _req: HttpRequest,
    bytes: web::Bytes,
) -> actix_web::Result<HttpResponse> {
    mavlink_post_with_schema(data, bytes, JsonSchema::Modern).await
}

#[api_v2_operation]
#[allow(clippy::await_holding_lock)]
/// Send a MAVLink message using the legacy 0.x JSON schema
pub async fn mavlink_post_legacy(
    data: web::Data<MAVLinkVehicleArcMutex>,
    _req: HttpRequest,
    bytes: web::Bytes,
) -> actix_web::Result<HttpResponse> {
    mavlink_post_with_schema(data, bytes, JsonSchema::Legacy).await
}

#[allow(clippy::await_holding_lock)]
async fn mavlink_post_with_schema(
    data: web::Data<MAVLinkVehicleArcMutex>,
    bytes: web::Bytes,
    schema: JsonSchema,
) -> actix_web::Result<HttpResponse> {
    let json_string = match String::from_utf8(bytes.to_vec()) {
        Ok(content) => content,
        Err(err) => {
            return not_found_response(format!("Failed to parse input as UTF-8 string: {err:?}"))
                .await;
        }
    };

    debug!("MAVLink post received: {json_string}");

    let content = match parse_incoming_message(&json_string, schema) {
        Ok(content) => content,
        Err(error) => return not_found_response(error).await,
    };

    match data.lock().unwrap().send(&content.header, &content.message) {
        Ok(_result) => {
            websocket_manager::send(&content);
            data::update((content.header, content.message));
            HttpResponse::Ok().await
        }
        Err(err) => not_found_response(format!("Failed to send message: {err:?}")).await,
    }
}

pub fn parse_incoming_message(
    json_string: &str,
    schema: JsonSchema,
) -> Result<data::MAVLinkMessage<mavlink::dialects::ardupilotmega::MavMessage>, String> {
    let value: serde_json::Value =
        json5::from_str(json_string).map_err(|error| format!("Failed to parse JSON5: {error}"))?;
    let value = match schema {
        JsonSchema::Modern => value,
        JsonSchema::Legacy => legacy::legacy_to_modern(&value)?,
    };
    serde_json::from_value(value)
        .map_err(|error| format!("Failed to parse message, not a valid MAVLinkMessage: {error:?}"))
}

#[api_v2_operation]
/// Websocket used to receive and send MAVLink messages asynchronously
pub async fn websocket(
    req: HttpRequest,
    query: web::Query<WebsocketQuery>,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    websocket_with_schema(req, query, stream, JsonSchema::Modern).await
}

#[api_v2_operation]
/// Websocket using the legacy 0.x JSON schema
pub async fn websocket_legacy(
    req: HttpRequest,
    query: web::Query<WebsocketQuery>,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    websocket_with_schema(req, query, stream, JsonSchema::Legacy).await
}

async fn websocket_with_schema(
    req: HttpRequest,
    query: web::Query<WebsocketQuery>,
    stream: web::Payload,
    schema: JsonSchema,
) -> Result<HttpResponse, actix_web::Error> {
    let query = query.into_inner();
    let filter = match query.filter {
        Some(filter) => filter,
        _ => ".*".to_owned(),
    };

    debug!("New websocket with filter {filter:#?} schema={schema:?}");

    let actor = WebsocketActor::new(filter, query.system_id, query.component_id, schema);
    ws::start(actor, &req, stream)
}

async fn not_found_response(message: String) -> actix_web::Result<HttpResponse> {
    HttpResponse::NotFound()
        .content_type("application/json")
        .body(message)
        .await
}

async fn ok_response(message: String) -> actix_web::Result<HttpResponse> {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(message)
        .await
}
