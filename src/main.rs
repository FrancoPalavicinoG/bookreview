#![allow(clippy::needless_return)]
#[macro_use] extern crate rocket;

use rocket::{Rocket, Build};
use rocket::http::Method;
use rocket_dyn_templates::Template;
use rocket_cors::{CorsOptions, AllowedOrigins, AllowedHeaders};

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

// ------- Rutas base -------
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
        .mount("/", routes![health])
        .mount("/authors", routes::authors::routes())
        .mount("/books", routes::books::routes())
}