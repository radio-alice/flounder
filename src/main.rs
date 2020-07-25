use serde::Deserialize;
use std::path::Path;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use gmi2html;
mod db;

#[derive(Deserialize)]
struct Config {
    db_path: &'static Path,
    file_directory: &'static Path,
    tls_enabled: bool,
    server_name: &'static str,
}

async fn index() -> impl Responder {
    HttpResponse::Ok().body(&gmi2html::convert("Hello world!"))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // db::initialize_tables().unwrap();
    // parse arguments using light library
    // initialize config
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
