use dotenvy::dotenv;
use std::env;

use mongodb::{
    bson::doc,
    options::{ClientOptions, IndexOptions},
    Client, Database, IndexModel,
};

pub struct AppState {
    pub db: Database,
}

pub async fn ensure_indexes(db: &Database) -> mongodb::error::Result<()> {
    // ========== BOOKS ==========
    let books = db.collection::<mongodb::bson::Document>("books");

    // Si ya tenías un text index solo en "summary", conviene reemplazarlo por uno combinado:
    // keys: { title: "text", summary: "text" }
    let text_idx = IndexModel::builder()
        .keys(doc! { "title": "text", "summary": "text" })
        .build();
    let _ = books.create_index(text_idx).await?;

    // author_id (para filtrar por autor)
    let author_idx = IndexModel::builder()
        .keys(doc! { "author_id": 1 })
        .build();
    let _ = books.create_index(author_idx).await?;

    // total_sales (para top 50 más vendidos)
    let total_sales_idx = IndexModel::builder()
        .keys(doc! { "total_sales": -1 })
        .build();
    let _ = books.create_index(total_sales_idx).await?;

    // opcional: publicación (si filtras/ordenas por fecha)
    let pub_date_idx = IndexModel::builder()
        .keys(doc! { "publication_date": 1 })
        .build();
    let _ = books.create_index(pub_date_idx).await?;

    // ========== REVIEWS ==========
    let reviews = db.collection::<mongodb::bson::Document>("reviews");
    let reviews_idx = IndexModel::builder()
        .keys(doc! { "book_id": 1, "score": -1, "up_votes": -1 })
        .build();
    let _ = reviews.create_index(reviews_idx).await?;

    // ========== SALES ==========
    let sales = db.collection::<mongodb::bson::Document>("sales");

    // Único por (book_id, year)
    let sales_unique = IndexModel::builder()
        .keys(doc! { "book_id": 1, "year": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build();
    let _ = sales.create_index(sales_unique).await?;

    // Index por año (tendencias por año)
    let year_idx = IndexModel::builder()
        .keys(doc! { "year": 1 })
        .build();
    let _ = sales.create_index(year_idx).await?;

    // opcional: si haces muchas sumas por libro
    let sales_book_idx = IndexModel::builder()
        .keys(doc! { "book_id": 1 })
        .build();
    let _ = sales.create_index(sales_book_idx).await?;

    Ok(())
}

pub async fn init_db() -> AppState {
    dotenv().ok();
    let uri = env::var("MONGO_URI").expect("MONGO_URI not set in .env");
    let dbname = env::var("DB_NAME").unwrap_or_else(|_| "bookreview".into());

    let mut opts = ClientOptions::parse(&uri).await.expect("Invalid MONGO_URI");
    opts.app_name = Some("bookreview".into());

    let client = Client::with_options(opts).expect("Cannot create Mongo client");
    let db = client.database(&dbname);

    if let Err(e) = ensure_indexes(&db).await {
        eprintln!("Failed to create indexes: {e}");
    }

    AppState { db }
}