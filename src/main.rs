#![allow(clippy::needless_return)]
#[macro_use] extern crate rocket;

use rocket::{Rocket, Build, State};
use rocket::http::Method;
use rocket::fs::FileServer;              // <-- NUEVO: para servir estáticos
use rocket_dyn_templates::Template;
use rocket_cors::{CorsOptions, AllowedOrigins, AllowedHeaders};
use serde_json::json;
use rocket::form::{Form, FromForm};
use rocket::response::Redirect;

use std::time::Duration;
use futures_util::stream::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId};

// Declaramos módulos
mod config;                              // <-- NUEVO
mod cache;                               // <-- NUEVO
mod search;                              // <-- NUEVO
mod db;
mod models;
mod routes {
    pub mod authors;
    pub mod books;
    pub mod reviews;
    pub mod sales;
    pub mod tables;
    pub mod search;
}

// Search form structure
#[derive(FromForm)]
struct SearchForm {
    q: String,
    page: Option<i64>,
}

// ------- Rutas base -------
#[get("/")]
async fn home(state: &State<db::AppState>) -> Template {   
    let authors_summary = match state.get_authors_summary_cached().await {
        Ok(summaries) => summaries,
        Err(e) => {
            eprintln!("Error getting authors summary: {}", e);
            vec![]
        }
    };

    let top_rated_books = match state.get_top_rated_books().await {
        Ok(books) => books,
        Err(e) => {
            eprintln!("Error getting top rated books: {}", e);
            vec![]
        }
    };

    let top_selling_books = match state.get_top_selling_books().await {
        Ok(books) => books,
        Err(e) => {
            eprintln!("Error getting top selling books: {}", e);
            vec![]
        }
    };

    let context = json!({
        "authors": authors_summary,
        "top_rated_books": top_rated_books,
        "top_selling_books": top_selling_books,
        "search_results": null,
        "show_search_results": false
    });
    
    Template::render("home", &context)
}

#[get("/search?<q>&<page>")]
async fn search_route(q: String, page: Option<i64>, state: &State<db::AppState>) -> Template {
    let page = page.unwrap_or(1);
    let per_page = 10;

    let search_results = match state.search_books_cached(&q, page, per_page).await {
        Ok(results) => Some(results),
        Err(e) => {
            eprintln!("Error searching books: {}", e);
            None
        }
    };

    let authors_summary = match state.get_authors_summary().await {
        Ok(summaries) => summaries,
        Err(e) => {
            eprintln!("Error getting authors summary: {}", e);
            vec![]
        }
    };

    let top_rated_books = match state.get_top_rated_books().await {
        Ok(books) => books,
        Err(e) => {
            eprintln!("Error getting top rated books: {}", e);
            vec![]
        }
    };

    let top_selling_books = match state.get_top_selling_books().await {
        Ok(books) => books,
        Err(e) => {
            eprintln!("Error getting top selling books: {}", e);
            vec![]
        }
    };

    let context = json!({
        "authors": authors_summary,
        "top_rated_books": top_rated_books,
        "top_selling_books": top_selling_books,
        "search_results": search_results,
        "show_search_results": true
    });
    
    Template::render("home", &context)
}

/// GET /books/avg/<id>
#[get("/books/avg/<id>")]
async fn book_avg(id: &str, state: &State<db::AppState>) -> String {
    use mongodb::bson::oid::ObjectId;

    let oid = match ObjectId::parse_str(id) {
        Ok(o) => o,
        Err(_) => return "invalid id".into(),
    };

    match state.get_book_average_score_cached(&oid).await {
        Ok(avg) => format!("{:.4}", avg),
        Err(e) => {
            eprintln!("[avg] error for {id}: {e}");
            "error".into()
        }
    }
}

#[get("/health")]
fn health() -> &'static str {
    "ok"
}

// CORS abierto para desarrollo.
fn cors() -> rocket_cors::Cors {
    let allowed_origins = AllowedOrigins::all();

    CorsOptions {
        allowed_origins,
        allowed_methods: vec![
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Delete,
            Method::Patch,
            Method::Options,
        ].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&[
            "Content-Type",
            "Accept",
            "Authorization",
        ]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("error building CORS")
}

#[launch]
async fn rocket() -> Rocket<Build> {
    // 1) Leemos config para decidir si montamos estáticos
    let cfg = crate::config::AppConfig::from_env();

    // 2) Creamos el estado (db::init_db() ya usa AppConfig por dentro
    //    y devuelve AppState con NoopCache/NoopSearch si no hay URLs configuradas)
    let state = db::init_db().await;

    // 3) Construimos Rocket y montamos rutas
    let mut app = rocket::build()
        .manage(state)
        .attach(Template::fairing())
        .attach(cors())
        .mount("/", routes![home, search_route, health, book_avg])
        .mount("/authors", routes::authors::routes())
        .mount("/books", routes::books::routes())
        .mount("/reviews", routes::reviews::routes())
        .mount("/sales", routes::sales::routes());

    // 4) Si elegiste servir estáticos desde la app (SERVE_STATIC=app),
    //    montamos /static -> {STATIC_DIR}
    if cfg.serve_static_from_app {
        app = app.mount("/static", FileServer::from(&cfg.static_dir));
    }

    app
}