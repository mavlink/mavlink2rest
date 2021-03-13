use actix_web::{HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::data;

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

pub fn root(req: HttpRequest) -> HttpResponse {
    let index = std::include_str!(concat!("html/", "index.html"));
    let vue = std::include_str!(concat!("html/", "vue.js"));
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
