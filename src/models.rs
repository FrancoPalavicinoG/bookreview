use serde::{Deserialize, Serialize};
use mongodb::bson::{oid::ObjectId};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Author {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub date_of_birth: Option<String>,
    pub country: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Book {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub author_id: ObjectId,            // relación 1–N (Book -> Author)
    pub title: String,
    pub summary: Option<String>,
    pub publication_date: Option<String>,
    pub total_sales: Option<i64>, 
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Review {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub book_id: ObjectId,   // relación N:1 con Book
    pub text: String,
    pub score: i32,          // 1..5
    pub up_votes: i64,       // >= 0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sale {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub book_id: ObjectId, // N:1 con Book
    pub year: i32,         // e.g. 2020
    pub units: i64,        // ventas ese año
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorSummary {
    pub author_id: ObjectId,
    pub name: String,
    pub published_books: i64,
    pub average_score: f64,
    pub total_sales: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopRatedBook {
    pub book_id: ObjectId,
    pub title: String,
    pub author_name: String,
    pub average_score: f64,
    pub total_reviews: i64,
    pub highest_rated_review: Option<ReviewWithScore>,
    pub lowest_rated_review: Option<ReviewWithScore>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewWithScore {
    pub text: String,
    pub score: i32,
    pub up_votes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopSellingBook {
    pub book_id: ObjectId,
    pub title: String,
    pub author_name: String,
    pub publication_date: Option<String>,
    pub book_total_sales: i64,
    pub author_total_sales: i64,
    pub was_top_5_in_publication_year: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub book_id: ObjectId,
    pub title: String,
    pub author_name: String,
    pub summary: Option<String>,
    pub publication_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedSearchResults {
    pub results: Vec<SearchResult>,
    pub current_page: i64,
    pub total_pages: i64,
    pub total_results: i64,
    pub has_next: bool,
    pub has_prev: bool,
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookSearchDoc {
    pub book_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub author_name: String,
    pub publication_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewSearchDoc {
    pub review_id: String,
    pub book_id: String,
    pub text: String,
    pub score: i32,
}