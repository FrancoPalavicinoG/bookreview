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

async fn ensure_indexes(db: &Database) -> mongodb::error::Result<()> {
    // books
    let books = db.collection::<mongodb::bson::Document>("books");
    let text_idx = IndexModel::builder().keys(doc! { "summary": "text" }).build();
    let _ = books.create_index(text_idx).await?;

    let author_idx = IndexModel::builder().keys(doc! { "author_id": 1 }).build();
    let _ = books.create_index(author_idx).await?;

    // reviews
    let reviews = db.collection::<mongodb::bson::Document>("reviews");
    let reviews_idx = IndexModel::builder()
        .keys(doc! { "book_id": 1, "score": -1, "up_votes": -1 })
        .build();
    let _ = reviews.create_index(reviews_idx).await?;

    // sales
    let sales = db.collection::<mongodb::bson::Document>("sales");
    let sales_unique = IndexModel::builder()
        .keys(doc! { "book_id": 1, "year": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build();
    let _ = sales.create_index(sales_unique).await?;

    let year_idx = IndexModel::builder().keys(doc! { "year": 1 }).build();
    let _ = sales.create_index(year_idx).await?;

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