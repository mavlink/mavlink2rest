use std::sync::{Arc, Mutex};

use chrono;
use chrono::offset::TimeZone;

use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Responder};

use mavlink::Message;

use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MavlinkMessage {
    pub header: mavlink::MavHeader,
    pub message: mavlink::common::MavMessage,
}

#[derive(Deserialize, Debug, Default)]
pub struct JsonConfiguration {
    pretty: Option<bool>,
}

pub struct API {
    messages: Arc<Mutex<serde_json::value::Value>>,
}

impl API {
    pub fn new(messages: Arc<Mutex<serde_json::value::Value>>) -> API {
        API { messages }
    }

    pub fn root_page(&self) -> impl Responder {
        let messages = Arc::clone(&self.messages);
        let messages = messages.lock().unwrap();
        let mut html_list_content = String::new();
        let now = chrono::Local::now();
        for key in messages["mavlink"].as_object().unwrap().keys() {
            let frequency = messages["mavlink"][&key]["message_information"]["frequency"]
                .as_f64()
                .unwrap_or(0.0);
            let last_time = now
                - chrono::Local
                    .datetime_from_str(
                        &messages["mavlink"][&key]["message_information"]["time"]["last_message"]
                            .to_string(),
                        "\"%+\"",
                    )
                    .unwrap_or(now);
            html_list_content = format!(
                "{0} <li> <a href=\"mavlink/{1}\">mavlink/{1}</a> ({2:.2}Hz - last update {3:#?}s ago) </li>",
                html_list_content,
                key,
                frequency,
                last_time.num_milliseconds() as f64/1e3
            );
        }
        // Remove guard after clone
        std::mem::drop(messages);

        let html_list = format!("<ul> {} </ul>", html_list_content);

        let html = format!(
            "{} - {} - {}<br>By: {}<br>
            Check the <a href=\"\\mavlink\">mavlink path</a> for the data<br>
            You can also check nested paths: <a href=\"mavlink/HEARTBEAT/mavtype/type\">mavlink/HEARTBEAT/mavtype/type</a><br>
            <br>
            List of available paths:
            {}
            ",
            env!("CARGO_PKG_NAME"),
            env!("VERGEN_SEMVER"),
            env!("VERGEN_BUILD_DATE"),
            env!("CARGO_PKG_AUTHORS"),
            html_list,
        );
        HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body(html)
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

        if query.pretty.is_some() && query.pretty.unwrap() {
            return HttpResponse::Ok()
                .content_type("application/json")
                .body(serde_json::to_string_pretty(&final_result).unwrap());
        }
        return HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&final_result).unwrap());
    }

    pub fn mavlink_helper_page(&self, req: HttpRequest) -> impl Responder {
        let query = web::Query::<JsonConfiguration>::from_query(req.query_string())
            .unwrap_or_else(|_| web::Query(Default::default()));

        let url_path = req.path().to_string();
        let message_name = url_path.split('/').last();

        let result: Result<mavlink::common::MavMessage, &'static str> = match message_name {
            Some(message_name) => match mavlink::common::MavMessage::message_id_from_name(message_name) {
                Ok(name) => mavlink::Message::default_message_from_id(name),
                Err(error) => Err(error),
            },
            _ => Err("Path should contain a valid name."),
        };

        match result {
            Ok(result) => {
                if query.pretty.is_some() && query.pretty.unwrap() {
                    return HttpResponse::Ok()
                        .content_type("application/json")
                        .body(serde_json::to_string_pretty(&result).unwrap());
                }

                return HttpResponse::Ok()
                        .content_type("application/json")
                        .body(serde_json::to_string(&result).unwrap());
            }
            Err(content) => {
                if query.pretty.is_some() && query.pretty.unwrap() {
                    return HttpResponse::NotFound()
                        .content_type("application/json")
                        .body(serde_json::to_string_pretty(&content).unwrap());
                }

                return HttpResponse::NotFound()
                        .content_type("application/json")
                        .body(serde_json::to_string(&content).unwrap());
            }
        }
    }

    pub fn mavlink_post(&mut self, req: web::Json<MavlinkMessage>) -> MavlinkMessage {
        req.into_inner()
    }
}
