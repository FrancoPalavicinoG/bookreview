#![allow(clippy::needless_return)]
#[macro_use] extern crate rocket;

use rocket::{Rocket, Build, State};
use rocket::http::Method;
use rocket_dyn_templates::Template;
use rocket_cors::{CorsOptions, AllowedOrigins, AllowedHeaders};
use serde_json::json;
use rocket::form::{Form, FromForm};
use rocket::response::Redirect;

// Declaramos módulos (archivos) aunque estén vacíos por ahora.
// No los usamos todavía para evitar errores de compilación.
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
        "search_results": null,
        "show_search_results": false
    });
    
    Template::render("home", &context)
}

#[get("/search?<q>&<page>")]
async fn search(q: String, page: Option<i64>, state: &State<db::AppState>) -> Template {
    let page = page.unwrap_or(1);
    let per_page = 10;

    let search_results = match state.search_books(&q, page, per_page).await {
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

#[get("/health")]
fn health() -> &'static str {
    "ok"
}

// CORS abierto para desarrollo.
// Ajusta AllowedOrigins si quieres restringirlo.
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
    // Requiere que implementemos db::init_db() en el siguiente paso.
    let state = db::init_db().await;

    rocket::build()
        .manage(state)
        // Templates Tera (los usaremos cuando hagamos las vistas)
        .attach(Template::fairing())
        // CORS dev
        .attach(cors())
        // Rutas mínimas por ahora (agregaremos las demás más adelante)
        .mount("/", routes![home, search, health])
        .mount("/authors", routes::authors::routes())
        .mount("/books", routes::books::routes())
        .mount("/reviews", routes::reviews::routes())
        .mount("/sales", routes::sales::routes())
}