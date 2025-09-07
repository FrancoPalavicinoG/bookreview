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
use tokio::task;

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
    image_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BookDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    author_id: ObjectId,
    title: String,
    summary: Option<String>,
    publication_date: Option<String>,
    total_sales: Option<i64>,
    cover_image_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ReviewDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    book_id: ObjectId,
    text: String,
    score: i32,
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

// Fast bulk insert with optimal batch size
async fn fast_bulk_insert<T>(collection: &Collection<T>, docs: Vec<T>) -> Result<()>
where
    T: serde::Serialize + std::marker::Unpin + std::marker::Send + std::marker::Sync + Clone,
{
    if docs.is_empty() {
        return Ok(());
    }

    // Use larger batch size for maximum performance (MongoDB's default max is 16MB)
    const BATCH_SIZE: usize = 5000;
    
    for chunk in docs.chunks(BATCH_SIZE) {
        collection.insert_many(chunk.to_vec()).ordered(false).await?;
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let uri = std::env::var("MONGO_URI").expect("MONGO_URI not set");
    let db_name = std::env::var("DB_NAME").expect("DB_NAME not set");

    // Fixed requirements
    const AUTHORS_COUNT: usize = 50;
    const BOOKS_COUNT: usize = 300;
    const MIN_REVIEWS_PER_BOOK: i32 = 1;
    const MAX_REVIEWS_PER_BOOK: i32 = 10;
    const SALES_YEARS_PER_BOOK: i32 = 5;

    let total_start = Instant::now();

    // Optimized MongoDB connection for bulk operations
    let mut client_opts = ClientOptions::parse(&uri).await?;
    client_opts.app_name = Some("bookreview-fast-seeder".into());
    client_opts.max_pool_size = Some(20);
    client_opts.min_pool_size = Some(10);
    client_opts.max_idle_time = Some(std::time::Duration::from_secs(60));
    let client = Client::with_options(client_opts)?;
    let db = client.database(&db_name);

    let authors_c: Collection<AuthorDoc> = db.collection("authors");
    let books_c: Collection<BookDoc> = db.collection("books");
    let reviews_c: Collection<ReviewDoc> = db.collection("reviews");
    let sales_c: Collection<SaleDoc> = db.collection("sales");

    logln("🚀 Starting fast seeder...");

    // Step 1: Clear all collections in parallel
    logln("🧹 Clearing collections...");
    let clear_start = Instant::now();
    let (r1, r2, r3, r4) = tokio::join!(
        authors_c.delete_many(doc! {}),
        books_c.delete_many(doc! {}),
        reviews_c.delete_many(doc! {}),
        sales_c.delete_many(doc! {})
    );
    r1?; r2?; r3?; r4?;
    logln(&format!("✅ Cleared all collections in {:?}", clear_start.elapsed()));

    // Step 2: Generate all data in memory (super fast)
    logln("📊 Generating all data...");
    let gen_start = Instant::now();

    let countries = [
        "US", "UK", "CA", "AR", "CL", "MX", "BR", "CO", "ES", "FR", 
        "DE", "IT", "JP", "KR", "CN", "IN", "NG", "ZA", "SE", "NO",
    ];
    let mut rng = rand::thread_rng();

    // Generate authors with pre-assigned IDs
    let mut authors: Vec<AuthorDoc> = Vec::with_capacity(AUTHORS_COUNT);
    for _ in 0..AUTHORS_COUNT {
        let name: String = Name(EN).fake();
        let description: String = Sentence(6..14).fake();
        
        let year = rng.gen_range(1930..=1995);
        let month = rng.gen_range(1..=12);
        let day = rng.gen_range(1..=28);
        let dob = Utc
            .with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()
            .map(|dt| dt.format("%Y-%m-%d").to_string());

        let country = Some(countries.choose(&mut rng).unwrap().to_string());

        authors.push(AuthorDoc {
            id: Some(ObjectId::new()),
            name,
            date_of_birth: dob,
            country,
            description: Some(description),
            image_path: None,  // Default to None for seeded authors
        });
    }

    // Generate books with balanced distribution across authors
    let mut books: Vec<BookDoc> = Vec::with_capacity(BOOKS_COUNT);
    let books_per_author = BOOKS_COUNT / AUTHORS_COUNT;
    let extra_books = BOOKS_COUNT % AUTHORS_COUNT;

    let mut book_index = 0;
    for (author_idx, author) in authors.iter().enumerate() {
        let author_id = author.id.unwrap();
        
        // Distribute books evenly, with some authors getting one extra book
        let books_for_this_author = if author_idx < extra_books {
            books_per_author + 1
        } else {
            books_per_author
        };

        for _ in 0..books_for_this_author {
            let book_id = ObjectId::new();
            
            let mut title: String = Sentence(2..5).fake();
            if title.ends_with('.') { title.pop(); }
            
            let summary: String = Sentence(10..20).fake();
            
            let pub_year = rng.gen_range(1990..=2024);
            let pub_month = rng.gen_range(1..=12);
            let pub_day = rng.gen_range(1..=28);
            let pub_date = Utc
                .with_ymd_and_hms(pub_year, pub_month, pub_day, 0, 0, 0)
                .single()
                .map(|dt| dt.format("%Y-%m-%d").to_string());

            books.push(BookDoc {
                id: Some(book_id),
                author_id,
                title,
                summary: Some(summary),
                publication_date: pub_date,
                total_sales: Some(0), // Will be calculated from sales
                cover_image_path: None,  // Default to None for seeded books
            });
            
            book_index += 1;
            if book_index >= BOOKS_COUNT {
                break;
            }
        }
        
        if book_index >= BOOKS_COUNT {
            break;
        }
    }

    // Generate reviews and sales for all books
    let mut reviews: Vec<ReviewDoc> = Vec::new();
    let mut sales: Vec<SaleDoc> = Vec::new();
    
    // Pre-allocate with estimated capacity
    let avg_reviews_per_book = (MIN_REVIEWS_PER_BOOK + MAX_REVIEWS_PER_BOOK) / 2;
    reviews.reserve(BOOKS_COUNT * avg_reviews_per_book as usize);
    sales.reserve(BOOKS_COUNT * SALES_YEARS_PER_BOOK as usize);

    for book in &books {
        let book_id = book.id.unwrap();
        
        // Generate reviews (1-10 per book)
        let num_reviews = rng.gen_range(MIN_REVIEWS_PER_BOOK..=MAX_REVIEWS_PER_BOOK);
        for _ in 0..num_reviews {
            let text: String = Sentence(8..20).fake();
            let score = rng.gen_range(1..=5);
            let up_votes = rng.gen_range(0..=200);
            
            reviews.push(ReviewDoc {
                id: Some(ObjectId::new()),
                book_id,
                text,
                score,
                up_votes,
            });
        }
        
        // Generate exactly 5 years of sales per book
        let pub_year = book.publication_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse::<i32>().ok())
            .unwrap_or(2020);
        
        for year_offset in 0..SALES_YEARS_PER_BOOK {
            let sale_year = pub_year + year_offset;
            if sale_year <= 2025 { // Don't go beyond current year + 1
                let units = rng.gen_range(50..=5000);
                
                sales.push(SaleDoc {
                    id: Some(ObjectId::new()),
                    book_id,
                    year: sale_year,
                    units,
                });
            }
        }
    }

    // Update total_sales in books
    let mut book_sales_map = std::collections::HashMap::new();
    for sale in &sales {
        *book_sales_map.entry(sale.book_id).or_insert(0i64) += sale.units;
    }
    
    let mut updated_books = books;
    for book in &mut updated_books {
        if let Some(book_id) = book.id {
            book.total_sales = book_sales_map.get(&book_id).copied();
        }
    }

    logln(&format!(
        "✅ Generated {} authors, {} books, {} reviews, {} sales in {:?}",
        authors.len(), updated_books.len(), reviews.len(), sales.len(), gen_start.elapsed()
    ));

    // Step 3: Insert all data in parallel using separate tasks
    logln("💾 Inserting all data in parallel...");
    let insert_start = Instant::now();

    let (authors_result, books_result, reviews_result, sales_result) = tokio::join!(
        task::spawn({
            let authors_c = authors_c.clone();
            let authors = authors.clone();
            async move {
                logln("📝 Inserting authors...");
                fast_bulk_insert(&authors_c, authors).await
            }
        }),
        task::spawn({
            let books_c = books_c.clone();
            let books = updated_books.clone();
            async move {
                logln("📚 Inserting books...");
                fast_bulk_insert(&books_c, books).await
            }
        }),
        task::spawn({
            let reviews_c = reviews_c.clone();
            let reviews = reviews.clone();
            async move {
                logln("⭐ Inserting reviews...");
                fast_bulk_insert(&reviews_c, reviews).await
            }
        }),
        task::spawn({
            let sales_c = sales_c.clone();
            let sales = sales.clone();
            async move {
                logln("💰 Inserting sales...");
                fast_bulk_insert(&sales_c, sales).await
            }
        })
    );

    // Handle results
    authors_result??;
    books_result??;
    reviews_result??;
    sales_result??;

    logln(&format!("✅ All data inserted in {:?}", insert_start.elapsed()));
    logln(&format!("🎉 Seeding completed in {:?}", total_start.elapsed()));
    
    logln("\n📊 Final Statistics:");
    logln(&format!("  • Authors: {}", AUTHORS_COUNT));
    logln(&format!("  • Books: {}", updated_books.len()));
    logln(&format!("  • Reviews: {} (avg {:.1} per book)", reviews.len(), reviews.len() as f64 / updated_books.len() as f64));
    logln(&format!("  • Sales: {} ({} years per book)", sales.len(), SALES_YEARS_PER_BOOK));

    Ok(())
}