#![allow(clippy::needless_return)]
#[macro_use] extern crate rocket;

use rocket::{Rocket, Build, State};
use rocket::http::Method;
use rocket_dyn_templates::Template;
use rocket_cors::{CorsOptions, AllowedOrigins, AllowedHeaders};
use serde_json::json;
use rocket::form::{Form, FromForm};
use crate::models::{Book, Review};

use std::sync::Arc;
use futures_util::stream::TryStreamExt;

// Módulos
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
    let authors_summary = state.get_authors_summary().await.unwrap_or_default();
    let top_rated_books = state.get_top_rated_books().await.unwrap_or_default();
    let top_selling_books = state.get_top_selling_books().await.unwrap_or_default();

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

    let search_results = state.search_books(&q, page, per_page).await.ok();

    let authors_summary = state.get_authors_summary().await.unwrap_or_default();
    let top_rated_books = state.get_top_rated_books().await.unwrap_or_default();
    let top_selling_books = state.get_top_selling_books().await.unwrap_or_default();

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

// CORS abierto para desarrollo
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
    // Inicializamos DB y SearchEngine
    let mut state = db::init_db().await;

    // Opcional: sincronizar todos los libros y reviews existentes a OpenSearch
    if !state.search.is::<search::NullSearchEngine>() {
        let books_collection = state.db.collection::<Book>("books");
        let mut cursor = books_collection.find(None, None).await.unwrap();
        while let Some(book) = cursor.try_next().await.unwrap() {
            state.search.upsert_book(&book, &"Autor Desconocido".to_string()).await.ok();
        }

        let reviews_collection = state.db.collection::<Review>("reviews");
        let mut cursor = reviews_collection.find(None, None).await.unwrap();
        while let Some(review) = cursor.try_next().await.unwrap() {
            state.search.upsert_review(&review).await.ok();
        }
    }

    rocket::build()
        .manage(state)
        .attach(Template::fairing())
        .attach(cors())
        .mount("/", routes![home, search, health])
        .mount("/authors", routes::authors::routes())
        .mount("/books", routes::books::routes())
        .mount("/reviews", routes::reviews::routes())
        .mount("/sales", routes::sales::routes())
}