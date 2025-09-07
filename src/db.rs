use dotenvy::dotenv;
use std::env;
use futures_util::stream::TryStreamExt;

use mongodb::{
    bson::doc,
    options::{ClientOptions, IndexOptions},
    Client, Database, IndexModel,
};
use crate::models::{AuthorSummary, TopRatedBook, TopSellingBook, SearchResult, PaginatedSearchResults, BookWithAuthor};

pub struct AppState {
    pub db: Database,
}

pub async fn ensure_indexes(db: &Database) -> mongodb::error::Result<()> {
    // ========== BOOKS ==========
    let books = db.collection::<mongodb::bson::Document>("books");

    let text_idx = IndexModel::builder()
        .keys(doc! { "title": "text", "summary": "text" })
        .build();
    let _ = books.create_index(text_idx).await?;

    let author_idx = IndexModel::builder()
        .keys(doc! { "author_id": 1 })
        .build();
    let _ = books.create_index(author_idx).await?;


    let total_sales_idx = IndexModel::builder()
        .keys(doc! { "total_sales": -1 })
        .build();
    let _ = books.create_index(total_sales_idx).await?;

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

    let year_idx = IndexModel::builder()
        .keys(doc! { "year": 1 })
        .build();
    let _ = sales.create_index(year_idx).await?;

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

impl AppState {
    pub async fn get_authors_summary(&self) -> mongodb::error::Result<Vec<AuthorSummary>> {
        let pipeline = vec![
            // Lookup books for each author
            doc! {
                "$lookup": {
                    "from": "books",
                    "localField": "_id",
                    "foreignField": "author_id",
                    "as": "books"
                }
            },
            // Unwind books to get individual book documents
            doc! {
                "$unwind": {
                    "path": "$books",
                    "preserveNullAndEmptyArrays": true
                }
            },
            // Lookup reviews for each book
            doc! {
                "$lookup": {
                    "from": "reviews",
                    "localField": "books._id",
                    "foreignField": "book_id",
                    "as": "reviews"
                }
            },
            // Lookup sales for each book
            doc! {
                "$lookup": {
                    "from": "sales",
                    "localField": "books._id",
                    "foreignField": "book_id",
                    "as": "sales"
                }
            },
            // Group by author to calculate aggregated values
            doc! {
                "$group": {
                    "_id": "$_id",
                    "name": { "$first": "$name" },
                    "published_books": {
                        "$sum": {
                            "$cond": [
                                { "$ne": ["$books", null] },
                                1,
                                0
                            ]
                        }
                    },
                    "total_reviews": { "$push": "$reviews" },
                    "total_sales_data": { "$push": "$sales" }
                }
            },
            // Calculate average score and total sales
            doc! {
                "$addFields": {
                    "all_scores": {
                        "$reduce": {
                            "input": "$total_reviews",
                            "initialValue": [],
                            "in": { "$concatArrays": ["$$value", "$$this"] }
                        }
                    },
                    "all_sales": {
                        "$reduce": {
                            "input": "$total_sales_data",
                            "initialValue": [],
                            "in": { "$concatArrays": ["$$value", "$$this"] }
                        }
                    }
                }
            },
            doc! {
                "$addFields": {
                    "average_score": {
                        "$cond": [
                            { "$gt": [{ "$size": "$all_scores" }, 0] },
                            { "$avg": "$all_scores.score" },
                            0.0
                        ]
                    },
                    "total_sales": {
                        "$sum": "$all_sales.units"
                    }
                }
            },
            doc! {
                "$project": {
                    "author_id": "$_id",
                    "name": 1,
                    "published_books": 1,
                    "average_score": { "$round": ["$average_score", 2] },
                    "total_sales": 1
                }
            },
            // Sort by name
            doc! {
                "$sort": {
                    "name": 1
                }
            }
        ];

        let authors_collection = self.db.collection::<mongodb::bson::Document>("authors");
        let cursor = authors_collection.aggregate(pipeline).await?;
        
        let documents: Vec<mongodb::bson::Document> = cursor.try_collect().await?;
        let mut summaries = Vec::new();
        
        for doc in documents {
            if let Ok(summary) = mongodb::bson::from_document::<AuthorSummary>(doc) {
                summaries.push(summary);
            }
        }
        
        Ok(summaries)
    }

    pub async fn get_top_rated_books(&self) -> mongodb::error::Result<Vec<TopRatedBook>> {
        let pipeline = vec![
            // Start with books
            doc! {
                "$lookup": {
                    "from": "reviews",
                    "localField": "_id",
                    "foreignField": "book_id",
                    "as": "reviews"
                }
            },
            // Only include books that have reviews
            doc! {
                "$match": {
                    "reviews": { "$ne": [] }
                }
            },
            // Calculate average score for each book
            doc! {
                "$addFields": {
                    "average_score": { "$avg": "$reviews.score" },
                    "total_reviews": { "$size": "$reviews" }
                }
            },
            // Sort by average score (highest first)
            doc! {
                "$sort": {
                    "average_score": -1,
                    "total_reviews": -1  // Secondary sort by number of reviews
                }
            },
            // Take top 10
            doc! {
                "$limit": 10
            },
            // Lookup author information
            doc! {
                "$lookup": {
                    "from": "authors",
                    "localField": "author_id",
                    "foreignField": "_id",
                    "as": "author"
                }
            },
            // Unwind author
            doc! {
                "$unwind": "$author"
            },
            // Add highest and lowest rated reviews
            doc! {
                "$addFields": {
                    "highest_rated_review": {
                        "$arrayElemAt": [
                            {
                                "$filter": {
                                    "input": "$reviews",
                                    "as": "review",
                                    "cond": {
                                        "$eq": [
                                            "$$review.score",
                                            { "$max": "$reviews.score" }
                                        ]
                                    }
                                }
                            },
                            0
                        ]
                    },
                    "lowest_rated_review": {
                        "$arrayElemAt": [
                            {
                                "$filter": {
                                    "input": "$reviews",
                                    "as": "review",
                                    "cond": {
                                        "$eq": [
                                            "$$review.score",
                                            { "$min": "$reviews.score" }
                                        ]
                                    }
                                }
                            },
                            0
                        ]
                    }
                }
            },
            doc! {
                "$project": {
                    "book_id": "$_id",
                    "title": 1,
                    "author_name": "$author.name",
                    "average_score": { "$round": ["$average_score", 2] },
                    "total_reviews": 1,
                    "highest_rated_review": {
                        "text": "$highest_rated_review.text",
                        "score": "$highest_rated_review.score",
                        "up_votes": "$highest_rated_review.up_votes"
                    },
                    "lowest_rated_review": {
                        "text": "$lowest_rated_review.text",
                        "score": "$lowest_rated_review.score",
                        "up_votes": "$lowest_rated_review.up_votes"
                    }
                }
            }
        ];

        let books_collection = self.db.collection::<mongodb::bson::Document>("books");
        let cursor = books_collection.aggregate(pipeline).await?;
        
        let documents: Vec<mongodb::bson::Document> = cursor.try_collect().await?;
        let mut top_books = Vec::new();
        
        for doc in documents {
            if let Ok(book) = mongodb::bson::from_document::<TopRatedBook>(doc) {
                top_books.push(book);
            }
        }
        
        Ok(top_books)
    }

    pub async fn get_top_selling_books(&self) -> mongodb::error::Result<Vec<TopSellingBook>> {
        let pipeline = vec![
            // Start with books
            doc! {
                "$lookup": {
                    "from": "sales",
                    "localField": "_id",
                    "foreignField": "book_id",
                    "as": "sales"
                }
            },
            // Calculate total sales for each book
            doc! {
                "$addFields": {
                    "book_total_sales": { "$sum": "$sales.units" },
                    "publication_year": {
                        "$cond": [
                            { "$ne": ["$publication_date", null] },
                            {
                                "$toInt": {
                                    "$arrayElemAt": [
                                        { "$split": ["$publication_date", "-"] },
                                        0
                                    ]
                                }
                            },
                            null
                        ]
                    }
                }
            },
            // Only include books with sales
            doc! {
                "$match": {
                    "book_total_sales": { "$gt": 0 }
                }
            },
            // Sort by total sales (highest first) and limit to top 50
            doc! {
                "$sort": {
                    "book_total_sales": -1
                }
            },
            doc! {
                "$limit": 50
            },
            // Lookup author information
            doc! {
                "$lookup": {
                    "from": "authors",
                    "localField": "author_id",
                    "foreignField": "_id",
                    "as": "author"
                }
            },
            doc! {
                "$unwind": "$author"
            },
            // Calculate author total sales (all their books)
            doc! {
                "$lookup": {
                    "from": "books",
                    "localField": "author._id",
                    "foreignField": "author_id",
                    "as": "author_books"
                }
            },
            doc! {
                "$lookup": {
                    "from": "sales",
                    "localField": "author_books._id",
                    "foreignField": "book_id",
                    "as": "author_sales"
                }
            },
            doc! {
                "$addFields": {
                    "author_total_sales": { "$sum": "$author_sales.units" }
                }
            },
            doc! {
                "$lookup": {
                    "from": "books",
                    "let": { "pub_year": "$publication_year" },
                    "pipeline": [
                        {
                            "$addFields": {
                                "book_pub_year": {
                                    "$cond": [
                                        { "$ne": ["$publication_date", null] },
                                        {
                                            "$toInt": {
                                                "$arrayElemAt": [
                                                    { "$split": ["$publication_date", "-"] },
                                                    0
                                                ]
                                            }
                                        },
                                        null
                                    ]
                                }
                            }
                        },
                        {
                            "$match": {
                                "$expr": { "$eq": ["$book_pub_year", "$$pub_year"] }
                            }
                        },
                        {
                            "$lookup": {
                                "from": "sales",
                                "localField": "_id",
                                "foreignField": "book_id",
                                "as": "book_sales"
                            }
                        },
                        {
                            "$addFields": {
                                "total_sales": { "$sum": "$book_sales.units" }
                            }
                        },
                        {
                            "$sort": { "total_sales": -1 }
                        },
                        {
                            "$limit": 5
                        },
                        {
                            "$project": { "_id": 1 }
                        }
                    ],
                    "as": "top_5_same_year"
                }
            },
            doc! {
                "$addFields": {
                    "was_top_5_in_publication_year": {
                        "$cond": [
                            { "$ne": ["$publication_year", null] },
                            {
                                "$in": [
                                    "$_id",
                                    { "$map": {
                                        "input": "$top_5_same_year",
                                        "as": "book",
                                        "in": "$$book._id"
                                    }}
                                ]
                            },
                            false
                        ]
                    }
                }
            },
            doc! {
                "$project": {
                    "book_id": "$_id",
                    "title": 1,
                    "author_name": "$author.name",
                    "publication_date": 1,
                    "book_total_sales": 1,
                    "author_total_sales": 1,
                    "was_top_5_in_publication_year": 1
                }
            }
        ];

        let books_collection = self.db.collection::<mongodb::bson::Document>("books");
        let cursor = books_collection.aggregate(pipeline).await?;
        
        let documents: Vec<mongodb::bson::Document> = cursor.try_collect().await?;
        let mut top_selling_books = Vec::new();
        
        for doc in documents {
            if let Ok(book) = mongodb::bson::from_document::<TopSellingBook>(doc) {
                top_selling_books.push(book);
            }
        }
        
        Ok(top_selling_books)
    }

    pub async fn search_books(&self, query: &str, page: i64, per_page: i64) -> mongodb::error::Result<PaginatedSearchResults> {
        let skip = (page - 1) * per_page;
        
        // Split the query into words and create search terms
        let search_terms: Vec<&str> = query.split_whitespace().collect();
        
        if search_terms.is_empty() {
            return Ok(PaginatedSearchResults {
                results: vec![],
                current_page: page,
                total_pages: 0,
                total_results: 0,
                has_next: false,
                has_prev: false,
                query: query.to_string(),
            });
        }

        // Create regex patterns for each search term
        let regex_patterns: Vec<mongodb::bson::Document> = search_terms
            .iter()
            .map(|term| doc! { 
                "$or": [
                    { "title": { "$regex": term, "$options": "i" } },
                    { "summary": { "$regex": term, "$options": "i" } }
                ]
            })
            .collect();

        let search_filter = doc! {
            "$and": regex_patterns
        };

        // Count total results
        let books_collection = self.db.collection::<mongodb::bson::Document>("books");
        let total_results = books_collection.count_documents(search_filter.clone()).await?;
        let total_pages = (total_results as f64 / per_page as f64).ceil() as i64;

        // Get paginated results
        let pipeline = vec![
            doc! { "$match": search_filter },
            doc! {
                "$lookup": {
                    "from": "authors",
                    "localField": "author_id",
                    "foreignField": "_id",
                    "as": "author"
                }
            },
            doc! { "$unwind": "$author" },
            doc! {
                "$project": {
                    "book_id": "$_id",
                    "title": 1,
                    "author_name": "$author.name",
                    "summary": 1,
                    "publication_date": 1
                }
            },
            doc! { "$sort": { "title": 1 } },
            doc! { "$skip": skip },
            doc! { "$limit": per_page }
        ];

        let cursor = books_collection.aggregate(pipeline).await?;
        let documents: Vec<mongodb::bson::Document> = cursor.try_collect().await?;
        
        let mut results = Vec::new();
        for doc in documents {
            if let Ok(result) = mongodb::bson::from_document::<SearchResult>(doc) {
                results.push(result);
            }
        }

        Ok(PaginatedSearchResults {
            results,
            current_page: page,
            total_pages,
            total_results: total_results as i64,
            has_next: page < total_pages,
            has_prev: page > 1,
            query: query.to_string(),
        })
    }

    // Get all authors for dropdown selection
    pub async fn get_all_authors(&self) -> mongodb::error::Result<Vec<crate::models::Author>> {
        let collection = self.db.collection::<crate::models::Author>("authors");
        let cursor = collection.find(doc! {}).await?;
        let authors: Vec<crate::models::Author> = cursor.try_collect().await?;
        Ok(authors)
    }

    // Get all books with author names for dropdown selection
    pub async fn get_all_books_with_authors(&self) -> mongodb::error::Result<Vec<BookWithAuthor>> {
        let pipeline = vec![
            doc! {
                "$lookup": {
                    "from": "authors",
                    "localField": "author_id",
                    "foreignField": "_id",
                    "as": "author"
                }
            },
            doc! {
                "$unwind": "$author"
            },
            doc! {
                "$project": {
                    "_id": 1,
                    "title": 1,
                    "author_name": "$author.name"
                }
            },
            doc! {
                "$sort": {
                    "title": 1
                }
            }
        ];

        let collection = self.db.collection::<mongodb::bson::Document>("books");
        let cursor = collection.aggregate(pipeline).await?;
        let docs: Vec<mongodb::bson::Document> = cursor.try_collect().await?;
        
        let books: Vec<BookWithAuthor> = docs.iter()
            .filter_map(|doc| {
                Some(BookWithAuthor {
                    id: doc.get_object_id("_id").ok()?.to_string(),
                    title: doc.get_str("title").ok()?.to_string(),
                    author_name: doc.get_str("author_name").ok()?.to_string(),
                })
            })
            .collect();

        Ok(books)
    }
}