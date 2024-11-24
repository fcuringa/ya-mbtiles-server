mod app_conf;
mod auth_middleware;
mod error;

use crate::app_conf::AppState;
use crate::auth_middleware::auth_middleware;
use actix_web::middleware::from_fn;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use clap::{arg, Command};
use sanitize_filename::sanitize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use actix_web::web::Data;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

//const MONGO_URL: &str = "EUA4e0D9pWtBP2vrlS5Z";

#[get("/{mapName}/{x}/{y}/{z}")]
async fn get_mbtile_in_filesystem(req: HttpRequest) -> impl Responder {
    let app_data = req.app_data::<Data<AppState>>().unwrap();
    let webroot = app_data.webroot.as_str();

    let path = req.match_info().get("mapName").unwrap();
    let x: i32 = req.match_info().get("x").unwrap().parse().unwrap();
    let y: i32 = req.match_info().get("y").unwrap().parse().unwrap();
    let z: i32 = req.match_info().get("z").unwrap().parse().unwrap();

    let projected_y = (1 << z) - 1 - y;

    let filename = sanitize(path);
    let file_path = PathBuf::from(webroot)
        .join(filename)
        .join(x.to_string())
        .join(projected_y.to_string())
        .join(z.to_string());

    if file_path.exists() {
        match File::open(&file_path).await {
            Ok(file) => HttpResponse::Ok().streaming(ReaderStream::new(file)),
            Err(_) => HttpResponse::InternalServerError().body("Could not read file"),
        }
    } else {
        HttpResponse::NotFound().body("File not found")
    }
}

#[actix_web::main]
async fn main() {
    let matches = Command::new("Yet another MBTiles server")
        .version("0.1")
        .author("Florian Curinga")
        .about("Serves MBTiles really fast. Supports custom authentication and using filesystem as storage backend.")
        .arg(arg!(--port [PORT] "Port to listen on").default_value("3000"))
        .arg(arg!(--route [ROUTE] "Route prefix for serving MBTiles").default_value("/mbtiles"))
        .arg(arg!(--webroot [WEBROOT] "Webroot for serving tiles").default_value("./"))
        .arg(arg!(--authscript [AUTH_SCRIPT] "Python script for authentication").default_value("auth.py"))
        .arg(arg!(--authheaders [AUTH_HEADERS] "Request headers with authorization data, if you need several of them use commas to separate").default_value("Authorization"))
        .arg(arg!(--cachetime [CACHE_TIME] "Cache validity in seconds").default_value("3600"))
        .get_matches();

    // Get the port from the command line arguments
    let port = matches.get_one::<String>("port").unwrap().as_str();
    let prefix = matches.get_one::<String>("route").unwrap().as_str();
    let auth_script_path = matches.get_one::<String>("authscript").unwrap().as_str();
    let auth_headers = matches.get_one::<String>("authheaders").unwrap().as_str();
    let webroot = matches.get_one::<String>("webroot").unwrap().as_str();
    let cache_validity_s = matches.get_one::<String>("cachetime").unwrap().as_str().parse().unwrap();

    let bind_address = format!("0.0.0.0:{}", port);
    let prefix_str = format!("{}", prefix);

    let app_data = AppState {
        auth_script: auth_script_path.to_string(),
        webroot: webroot.to_string(),
        auth_cache: Arc::new(Mutex::new(Default::default())),
        cache_validity_seconds: cache_validity_s,
        auth_headers: auth_headers.split(',').map(|s| s.to_string()).collect()
    };
    let app_state = web::Data::new(app_data);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(from_fn(auth_middleware))
            .service(web::scope(&*prefix_str).service(get_mbtile_in_filesystem))
    });

    println!("Starting server on {}", bind_address);
    server.bind(bind_address).unwrap().run().await.unwrap();
}
