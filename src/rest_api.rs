use std::sync::{Arc, Mutex};

use actix_web::{web, HttpRequest, HttpResponse, Responder};

use mavlink::Message;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MavlinkMessage {
    pub header: mavlink::MavHeader,
    pub message: mavlink::ardupilotmega::MavMessage,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MavlinkMessageCommon {
    pub header: mavlink::MavHeader,
    pub message: mavlink::common::MavMessage,
}

#[derive(Deserialize, Debug, Default)]
pub struct JsonConfiguration {
    pretty: Option<bool>,
}

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

pub struct API {
    messages: Arc<Mutex<serde_json::value::Value>>,
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

pub fn root_page(req: HttpRequest) -> HttpResponse {
    let index = include_str!("html/index.html");
    let vue = include_str!("html/vue.js");
    let path = match req.match_info().query("filename") {
        "" | "index.html" => index,
        "vue.js" => vue,
        something => {
            return HttpResponse::NotFound()
                .content_type("text/plain")
                .body(format!("Page does not exist: {}", something));
        }
    };
    HttpResponse::Ok().content_type("text/html").body(path)
}

pub fn redirect_root_page() -> HttpResponse {
    let content = r#"<!DOCTYPE HTML><html><head><meta http-equiv="refresh" content="0; url=/index.html"></head></html>"#;
    HttpResponse::Ok().content_type("text/html").body(content)
}

impl API {
    pub fn new(messages: Arc<Mutex<serde_json::value::Value>>) -> API {
        API { messages }
    }

    pub fn parse_query<T: serde::ser::Serialize>(
        message: &T,
        query: &web::Query<JsonConfiguration>,
    ) -> String {
        let error_message =
            "Not possible to parse mavlink message, please report this issue!".to_string();
        if query.pretty.is_some() && query.pretty.unwrap() {
            return serde_json::to_string_pretty(&message).unwrap_or(error_message);
        }
        serde_json::to_string(&message).unwrap_or(error_message)
    }

    pub fn mavlink_page(&self, req: HttpRequest) -> impl Responder {
        let query = web::Query::<JsonConfiguration>::from_query(req.query_string())
            .unwrap_or_else(|_| web::Query(Default::default()));

        let url_path = req.path().to_string();
        let messages = Arc::clone(&self.messages);
        let messages = messages.lock().unwrap();
        let final_result = (*messages).pointer(&url_path);

        if final_result.is_none() {
            return HttpResponse::NotFound()
                .content_type("text/plain")
                .body(format!("No valid path: {}", &url_path));
        }

        let final_result = final_result.unwrap().clone();
        std::mem::drop(messages); // Remove guard after clone

        return HttpResponse::Ok()
            .content_type("application/json")
            .body(API::parse_query(&final_result, &query));
    }

    pub fn mavlink_helper_page(&self, req: HttpRequest) -> impl Responder {
        let query = web::Query::<JsonConfiguration>::from_query(req.query_string())
            .unwrap_or_else(|_| web::Query(Default::default()));

        let url_path = req.path().to_string();
        let message_name = url_path.split('/').last();

        let result: Result<mavlink::ardupilotmega::MavMessage, &'static str> = match message_name {
            Some(message_name) => {
                match mavlink::ardupilotmega::MavMessage::message_id_from_name(message_name) {
                    Ok(name) => mavlink::Message::default_message_from_id(name),
                    Err(error) => Err(error),
                }
            }
            _ => Err("Path should contain a valid name."),
        };

        match result {
            Ok(result) => {
                match result {
                    mavlink::ardupilotmega::MavMessage::common(msg) => {
                        let result = MavlinkMessageCommon {
                            header: mavlink::MavHeader::default(),
                            message: msg,
                        };

                        return HttpResponse::Ok()
                            .content_type("application/json")
                            .body(API::parse_query(&result, &query));
                    }
                    msg => {
                        let result = MavlinkMessage {
                            header: mavlink::MavHeader::default(),
                            message: msg,
                        };

                        return HttpResponse::Ok()
                            .content_type("application/json")
                            .body(API::parse_query(&result, &query));
                    }
                };
            }
            Err(content) => {
                return HttpResponse::NotFound()
                    .content_type("application/json")
                    .body(API::parse_query(&content, &query));
            }
        }
    }

    pub fn mavlink_post(&mut self, bytes: web::Bytes) -> Result<MavlinkMessage, std::io::Error> {
        let json_string = String::from_utf8(bytes.to_vec());

        if json_string.is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to parse input as UTF-8 string.",
            ));
        }
        let json_string = json_string.unwrap();
        self.extract_mavlink_from_string(&json_string)
    }

    pub fn extract_mavlink_from_string(
        &mut self,
        json_string: &String,
    ) -> Result<MavlinkMessage, std::io::Error> {
        let result = serde_json::from_str::<MavlinkMessageCommon>(&json_string);
        if let Ok(msg) = result {
            return Ok(MavlinkMessage {
                header: msg.header,
                message: mavlink::ardupilotmega::MavMessage::common(msg.message),
            });
        }
        let mut errors = Vec::new();
        errors.push(format!(
            "Failed to parse common message: {:#?}",
            result.err().unwrap()
        ));

        let result = serde_json::from_str::<MavlinkMessage>(&json_string);

        if let Ok(content) = result {
            // Remove serde_json::Error
            return Ok(content);
        }
        errors.push(format!(
            "Failed to parse ardupilotmega message: {:#?}",
            result.err().unwrap()
        ));

        let error = format!("{:?}", &errors);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            error.as_str(),
        ));
    }
}
