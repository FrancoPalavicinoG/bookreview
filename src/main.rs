#![allow(clippy::needless_return)]
#[macro_use] extern crate rocket;

use rocket::{Rocket, Build, State};
use rocket::http::Method;
use rocket::fs::FileServer;              // <-- NUEVO: para servir estáticos
use rocket_dyn_templates::Template;
use rocket_cors::{CorsOptions, AllowedOrigins, AllowedHeaders};
use serde_json::json;
use rocket::form::FromForm;
use rocket::request::FlashMessage;


use std::time::Duration;
use futures_util::stream::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId};

// Declaramos módulos
mod config;                              // <-- NUEVO
mod cache;                               // <-- NUEVO
mod search;                              // <-- NUEVO
mod db;
mod models;
mod static_files;
mod upload;
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


#[get("/upload")]
async fn upload_page(flash: Option<FlashMessage<'_>>, state: &State<db::AppState>) -> Template {
    let serve_static = static_files::should_serve_static();
    let uploads_dir = upload::get_uploads_dir();
    
    // Fetch all authors and books for dropdowns
    let authors = match state.get_all_authors().await {
        Ok(authors) => authors.into_iter().map(|author| {
            json!({
                "id": author.id.map(|id| id.to_string()).unwrap_or_default(),
                "name": author.name,
                "country": author.country
            })
        }).collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("Error fetching authors: {}", e);
            vec![]
        }
    };
    
    let books = match state.get_all_books_with_authors().await {
        Ok(books) => books,
        Err(e) => {
            eprintln!("Error fetching books: {}", e);
            vec![]
        }
    };
    
    let mut context = json!({
        "serve_static": serve_static,
        "uploads_dir": uploads_dir,
        "authors": authors,
        "books": books
    });
    
    if let Some(flash) = flash {
        context["flash"] = json!({
            "kind": flash.kind(),
            "message": flash.message()
        });
    }
    
    Template::render("upload", &context)
}

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
    // Initialize uploads directory
    if let Err(e) = upload::create_uploads_directory() {
        eprintln!("Warning: Failed to create uploads directory: {}", e);
    }

    let state = db::init_db().await;

    let mut rocket_builder = rocket::build()
        .manage(state)
        .attach(Template::fairing())
        .attach(cors())
        .mount("/", routes![home, search, health, upload_page, book_avg])
        .mount("/authors", routes::authors::routes())
        .mount("/books", routes::books::routes())
        .mount("/reviews", routes::reviews::routes())
        .mount("/sales", routes::sales::routes())
        .mount("/upload", upload::get_upload_routes());

    // Only serve static files if not behind reverse proxy
    if static_files::should_serve_static() {
        println!("Serving static files from application");
        rocket_builder = rocket_builder.mount("/static", static_files::get_static_routes());
    } else {
        println!("Static files will be served by reverse proxy");
    }

    rocket_builder
}