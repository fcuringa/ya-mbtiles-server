mod app_conf;
mod auth_middleware;
mod error;
mod mbtile_project;

use std::ops::Add;
use crate::app_conf::AppState;
use crate::auth_middleware::auth_middleware;
use actix_web::middleware::{from_fn, Logger};
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use clap::{arg, Command};
use sanitize_filename::sanitize;
use std::path::PathBuf;
use std::process::exit;
use std::ptr::null;
use std::sync::{Arc, Mutex};
use actix_cors::Cors;
use actix_web::web::{resource, Data};
use log::{error, info, warn};
use sqlite::{Connection, State};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use crate::mbtile_project::mbtile_project_y;

#[get("/{mapName}/{z}/{x}/{y}")]
async fn get_mbtile_in_filesystem(req: HttpRequest) -> impl Responder {
    let app_data = req.app_data::<Data<AppState>>().unwrap();
    let webroot = app_data.webroot.as_str();

    let path = req.match_info().get("mapName").unwrap();
    let x: i64 = req.match_info().get("x").unwrap().parse().unwrap();
    let y: i64 = req.match_info().get("y").unwrap().parse().unwrap();
    let z: i64 = req.match_info().get("z").unwrap().parse().unwrap();

    let projected_y = mbtile_project_y(x, y, z);

    let filename = sanitize(path);
    let file_path = PathBuf::from(webroot)
        .join(filename)
        .join(z.to_string())
        .join(projected_y.to_string())
        .join(x.to_string());

    let mut response = HttpResponse::Ok();

    if !app_data.cache_control_header.is_empty() {
        response.insert_header(("Cache-Control", app_data.cache_control_header.clone()));
    }

    if file_path.exists() {
        match File::open(&file_path).await {
            Ok(file) => response.streaming(ReaderStream::new(file)),
            Err(_) => HttpResponse::InternalServerError().body("Could not read file"),
        }
    } else {
        HttpResponse::NotFound().body("File not found")
    }
}

#[get("/{mapName}/{z}/{x}/{y}")]
async fn get_mbtile_sqlite(req: HttpRequest) -> impl Responder {
    let app_data = req.app_data::<Data<AppState>>().unwrap();
    let webroot = app_data.webroot.as_str();

    let path = req.match_info().get("mapName").unwrap();
    let x: i64 = req.match_info().get("x").unwrap().parse().unwrap();
    let y: i64 = req.match_info().get("y").unwrap().parse().unwrap();
    let z: i64 = req.match_info().get("z").unwrap().parse().unwrap();

    let projected_y = mbtile_project_y(x, y, z);

    let filename = sanitize(path);
    let file_path = PathBuf::from(webroot)
        .join(filename.add(".mbtiles"));
    let connection_res = sqlite::open(file_path);
    if connection_res.is_err() {
        HttpResponse::NotFound().body("Could not open file")
    } else {
        let connection = connection_res.unwrap();
        let query = "SELECT tile_data FROM tiles \
                            WHERE zoom_level=? AND tile_column=? and tile_row=?;";
        let statement_res = connection.prepare(query);
        if statement_res.is_err() {
            return HttpResponse::NotFound().body("Could not find tiles table")
        }
        let mut statement = statement_res.unwrap();
        statement.bind((1, z)).unwrap();
        statement.bind((2, x)).unwrap();
        statement.bind((3, projected_y)).unwrap();

        let query_res = statement.next();
        let data = statement.read::<Vec<u8>, _>("tile_data")
            .unwrap();

        let mut response = HttpResponse::Ok();

        if data.is_empty() {
            return HttpResponse::NotFound().body("Could not find tile data");
        }

        // Handle vector tiles
        if data[0] == 0x1f && data[1] == 0x8b {
            response.insert_header(("Content-Type", "application/x-protobuf"));
            response.insert_header(("Content-Encoding", "gzip"));
        }

        if !app_data.cache_control_header.is_empty() {
            response.insert_header(("Cache-Control", app_data.cache_control_header.clone()));
        }

        match query_res {
            Ok(State::Row) => response.body(data),
            Err(_) => HttpResponse::NotFound().body("Could not find tile data"),
            _ => {HttpResponse::InternalServerError().body("Could not read tile data")}
        }
    }
}

#[actix_web::main]
async fn main() {
    let matches = Command::new("Yet another MBTiles server")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Florian Curinga")
        .about("Serves MBTiles really fast. Supports custom authentication and using filesystem as storage backend.")
        .arg(arg!(--port [PORT] "Port to listen on").default_value("3000"))
        .arg(arg!(--route [ROUTE] "Route prefix for serving MBTiles").default_value("/mbtiles"))
        .arg(arg!(--webroot [WEBROOT] "Webroot for serving tiles").default_value("./example-data"))
        .arg(arg!(--tilesmode [TILES_MODE] "The serving mode for MBTiles, either 'filesystem' or 'mbtiles'. The former assumes the tile data is stored directly as individual files, the latter assumes a flat directory of MBTiles files.").default_value("mbtiles"))
        .arg(arg!(--authscript [AUTH_SCRIPT] "Python script for authentication").default_value("auth.py"))
        .arg(arg!(--authheaders [AUTH_HEADERS] "Request headers with authorization data, if you need several of them use commas to separate").default_value(""))
        .arg(arg!(--cachetime [CACHE_TIME] "Cache validity in seconds for the authentication result").default_value("3600"))
        .arg(arg!(--cachecontrolheader [CACHE_CONTROL_HEADER] "The Cache-Control header to be used in the responses").default_value("max-age=604800"))
        .get_matches();

    // Get the port from the command line arguments
    let port = matches.get_one::<String>("port").unwrap().as_str();
    let prefix = matches.get_one::<String>("route").unwrap().as_str();
    let auth_script_path = matches.get_one::<String>("authscript").unwrap().as_str();
    let auth_headers = matches.get_one::<String>("authheaders").unwrap().as_str();
    let webroot = matches.get_one::<String>("webroot").unwrap().as_str();
    let mbtiles_mode = matches.get_one::<String>("tilesmode").unwrap().as_str();
    let cache_control_header = matches.get_one::<String>("cachecontrolheader").unwrap().as_str();
    let cache_validity_s = matches.get_one::<String>("cachetime").unwrap().as_str().parse().unwrap();

    let bind_address = format!("0.0.0.0:{}", port);
    let prefix_str = format!("{}", prefix);

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let app_data = AppState {
        auth_script: auth_script_path.to_string(),
        webroot: webroot.to_string(),
        auth_cache: Arc::new(Mutex::new(Default::default())),
        cache_validity_seconds: cache_validity_s,
        cache_control_header: cache_control_header.to_string(),
        auth_headers: match auth_headers{
            "" => {
                warn!("No authentication was selected, client identity will not be checked");
                Vec::<String>::new()
            },
            _ => {
                let headers = auth_headers.split(',').map(|s| s.to_string()).collect();
                warn!("Those headers will be used for authentication: {}", auth_headers);
                headers
            }
        }
    };


    let app_state = web::Data::new(app_data);
    if mbtiles_mode == "filesystem" {
        let server = HttpServer::new(move || {
            App::new()
                .wrap(Cors::permissive())
                .wrap(Logger::default())
                .app_data(app_state.clone())
                .wrap(from_fn(auth_middleware))
                .service(web::scope(&*prefix_str)
                    .service(get_mbtile_in_filesystem))
        });
        info!("Starting server on {}, will get tile data using the 'filesystem' mode", bind_address);
        server.bind(bind_address).unwrap().run().await.unwrap();
    } else if mbtiles_mode == "mbtiles" {
        let server = HttpServer::new(move || {
            App::new()
                .wrap(Cors::permissive())
                .wrap(Logger::default())
                .app_data(app_state.clone())
                .wrap(from_fn(auth_middleware))
                .service(web::scope(&*prefix_str)
                    .service(get_mbtile_sqlite))
        });

        info!("Starting server on {}, will get tile data using the 'mbtiles' mode", bind_address);
        server.bind(bind_address).unwrap().run().await.unwrap();
    } else {
        error!("Invalid mode");
        exit(1);
    }

}
