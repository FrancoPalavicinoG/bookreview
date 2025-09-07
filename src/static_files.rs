use rocket::fs::{NamedFile, FileServer};
use rocket::{Route, routes, get};
use std::path::{Path, PathBuf};
use std::env;

// Static file serving route - only enabled when not behind reverse proxy
#[get("/static/<file..>")]
pub async fn serve_static(file: PathBuf) -> Option<NamedFile> {
    let serve_static = env::var("SERVE_STATIC_FILES")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    
    if !serve_static {
        return None;
    }

    let uploads_dir = env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".to_string());
    NamedFile::open(Path::new(&uploads_dir).join(file)).await.ok()
}

pub fn get_static_routes() -> Vec<Route> {
    routes![serve_static]
}

pub fn get_file_server() -> FileServer {
    let uploads_dir = env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".to_string());
    FileServer::from(uploads_dir)
}

pub fn should_serve_static() -> bool {
    env::var("SERVE_STATIC_FILES")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true)
}
