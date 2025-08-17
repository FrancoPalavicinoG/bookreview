use std::collections::{HashMap, HashSet};
use std::time::Instant;
use std::io::{self, Write};

use anyhow::Result;
use chrono::{TimeZone, Utc};
use dotenvy::dotenv;
use fake::faker::lorem::en::Sentence;
use fake::faker::name::raw::Name;
use fake::locales::EN;
use fake::Fake;
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::ClientOptions,
    Client, Collection,
};
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

fn logln(msg: &str) {
    println!("{msg}");
    let _ = io::stdout().flush();
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AuthorDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    date_of_birth: Option<String>,
    country: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BookDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    author_id: ObjectId,
    title: String,
    summary: Option<String>,
    publication_date: Option<String>, // YYYY-MM-DD
    total_sales: Option<i64>,         // será la suma de sales.units
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ReviewDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    book_id: ObjectId,
    text: String,
    score: i32,  // 1..5
    up_votes: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SaleDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    book_id: ObjectId,
    year: i32,
    units: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let uri = std::env::var("MONGO_URI").expect("MONGO_URI not set");
    let db_name = std::env::var("DB_NAME").expect("DB_NAME not set");

    let seed_authors: usize = std::env::var("SEED_AUTHORS").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    let max_books_per_author: i32 = std::env::var("MAX_BOOKS_PER_AUTHOR").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    let max_reviews_per_book: i32 = std::env::var("MAX_REVIEWS_PER_BOOK").ok().and_then(|v| v.parse().ok()).unwrap_or(6);
    let max_years_per_book: i32 = std::env::var("MAX_YEARS_PER_BOOK").ok().and_then(|v| v.parse().ok()).unwrap_or(5);

    let mut client_opts = ClientOptions::parse(&uri).await?;
    client_opts.app_name = Some("bookreview-seeder".into());
    let client = Client::with_options(client_opts)?;
    let db = client.database(&db_name);

    let authors_c: Collection<AuthorDoc> = db.collection("authors");
    let books_c: Collection<BookDoc> = db.collection("books");
    let reviews_c: Collection<ReviewDoc> = db.collection("reviews");
    let sales_c: Collection<SaleDoc> = db.collection("sales");

    // Countries 
    let countries = [
        "US","UK","CA","AR","CL","MX","BR","CO","ES","FR","DE","IT","JP","KR","CN","IN","NG","ZA","SE","NO",
    ];

    let mut rng = rand::thread_rng();


    // AUTHORS 
    logln("Generating authors...");
    let t0 = Instant::now();

    let mut a_uniques = HashSet::<String>::new();
    let mut author_docs: Vec<AuthorDoc> = Vec::new();

    while author_docs.len() < seed_authors {
        let name: String = Name(EN).fake();
        if !a_uniques.insert(name.clone()) {
            continue; // avoid exact duplicates
        }

        // Lorem description (6..14 words)
        let description: String = Sentence(6..14).fake();

        // Random date of birth (1930..=1995; day 1..=28 for simplicity)
        let year = rng.gen_range(1930..=1995);
        let month = rng.gen_range(1..=12);
        let day = rng.gen_range(1..=28);
        let dob = Utc
            .with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()
            .map(|dt| dt.format("%Y-%m-%d").to_string());

        // Random country
        let country = Some(countries.choose(&mut rng).unwrap().to_string());

        author_docs.push(AuthorDoc {
            id: Some(ObjectId::new()), // fijamos id para referenciar desde books
            name,
            date_of_birth: dob,
            country,
            description: Some(description),
        });
    }

    // Wipe & seed authors
    authors_c.delete_many(doc! {}).await?;
    let _ = authors_c.insert_many(author_docs.clone()).await?;
    logln(&format!("Seeded authors: {} (in {:?})", author_docs.len(), t0.elapsed()));

    // Mapa AuthorId -> AuthorDoc (por si se requiere)
    let mut authors_by_id: HashMap<ObjectId, AuthorDoc> = HashMap::new();
    for a in &author_docs {
        if let Some(id) = a.id {
            authors_by_id.insert(id, a.clone());
        }
    }

    logln("Generating books, reviews and sales...");
    let t1 = Instant::now();

    // Books
    let mut book_docs: Vec<BookDoc> = Vec::new();
    let mut review_docs: Vec<ReviewDoc> = Vec::new();
    let mut sale_docs: Vec<SaleDoc> = Vec::new();

    for a in &author_docs {
        let author_id = a.id.expect("author id");
        // 1..=max_books_per_author books per author
        let n_books = rng.gen_range(1..=max_books_per_author);
        for _ in 0..n_books {
            let book_id = ObjectId::new();

            // Title 2..5 words (Sentence adds a final '.', lo quitamos)
            let mut title: String = Sentence(2..5).fake();
            if title.ends_with('.') { title.pop(); }

            // Summary 10..20 words
            let summary: String = Sentence(10..20).fake();

            // Random publication date (1990..=2024)
            let y = rng.gen_range(1990..=2024);
            let m = rng.gen_range(1..=12);
            let d = rng.gen_range(1..=28);
            let pub_date = Utc
                .with_ymd_and_hms(y, m, d, 0, 0, 0)
                .single()
                .map(|dt| dt.format("%Y-%m-%d").to_string());

            // Sales: 1..=max_years_per_book years starting from pub year..pub+5 (<= 2025)
            let pub_year = y;
            let n_years = rng.gen_range(1..=max_years_per_book);
            let mut years = HashSet::<i32>::new();
            while years.len() < n_years as usize {
                let yr = rng.gen_range(pub_year..=(pub_year + 5)).min(2025);
                years.insert(yr);
            }

            let mut total_sales: i64 = 0;
            for yr in years {
                let units = rng.gen_range(50..=5000) as i64;
                total_sales += units;
                sale_docs.push(SaleDoc {
                    id: Some(ObjectId::new()),
                    book_id,
                    year: yr,
                    units,
                });
            }

            // Reviews: 0..=max_reviews_per_book
            let n_reviews = rng.gen_range(0..=max_reviews_per_book);
            for _ in 0..n_reviews {
                let text: String = Sentence(8..20).fake();
                let score = rng.gen_range(1..=5) as i32;
                let up_votes = rng.gen_range(0..=200) as i64;
                review_docs.push(ReviewDoc {
                    id: Some(ObjectId::new()),
                    book_id,
                    text,
                    score,
                    up_votes,
                });
            }

            book_docs.push(BookDoc {
                id: Some(book_id),
                author_id,
                title,
                summary: Some(summary),
                publication_date: pub_date,
                total_sales: Some(total_sales), // consistente con Sales recién creadas
            });
        }
    }

    logln(&format!(
        "Generated -> books: {} | reviews: {} | sales: {} (in {:?})",
        book_docs.len(), review_docs.len(), sale_docs.len(), t1.elapsed()
    ));

    logln("Clearing collections (reviews, sales, books)...");
    reviews_c.delete_many(doc! {}).await?;
    logln("Cleared reviews");
    sales_c.delete_many(doc! {}).await?;
    logln("Cleared sales");
    books_c.delete_many(doc! {}).await?;
    logln("Cleared books");

    logln("Inserting books...");
    let _ = books_c.insert_many(book_docs.clone()).await?;
    logln("Inserted books");
    logln("Inserting reviews...");
    let _ = reviews_c.insert_many(review_docs.clone()).await?;
    logln("Inserted reviews");
    logln("Inserting sales...");
    let _ = sales_c.insert_many(sale_docs.clone()).await?;
    logln("Inserted sales");

    logln("Seeding completed.");

    Ok(())
}