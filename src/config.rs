use std::env;

pub struct AppConfig {
    pub mongo_uri: String,
    pub db_name: String,
    pub cache_url: Option<String>,
    pub search_url: Option<String>,
    pub static_dir: String,
    pub serve_static_from_app: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv(); // ignora si no existe .env

        let get = |k: &str, d: &str| env::var(k).unwrap_or_else(|_| d.to_string());

        let serve_static_from_app = get("SERVE_STATIC", "app") == "app";

        Self {
            mongo_uri: get("MONGO_URI", "mongodb://localhost:27017"),
            db_name: get("DB_NAME", "bookreview_dev"),
            cache_url: env::var("CACHE_URL").ok(),
            search_url: env::var("SEARCH_URL").ok(),
            static_dir: get("STATIC_DIR", "./static"),
            serve_static_from_app,
        }
    }
}