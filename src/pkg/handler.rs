use anyhow::Context;
use ntex::web;
use ntex::web::HttpResponse;
use serde::Serialize;
use crate::pkg::err::AppError;
use serde_xml_rs::to_string;

#[derive(Serialize)]
struct Person {
    name: String,
    age: u32,
}

type HandlerResponse = Result<HttpResponse, AppError>;
#[web::get("/")]
async fn get() -> HandlerResponse {
    let person = Person {
        name: "John Doe".to_owned(),
        age: 30,
    };
    let xml = to_string(&person).context("")?;
    let res = HttpResponse::Ok().content_type("application/xml")
        .body(xml);
    Ok(res)
}