use anyhow::Result;
use async_trait::async_trait;
use crate::models::{Book, Review, BookSearchDoc, ReviewSearchDoc};

pub mod null_engine;
pub mod opensearch_engine;

pub use null_engine::NullSearchEngine;
pub use opensearch_engine::OpenSearchEngine;

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub id: String,
    pub kind: String,   // "book" | "review"
    pub score: f32,
    pub highlight: Option<String>,
}

#[async_trait]
pub trait SearchEngine: Send + Sync {
    async fn upsert_book(&self, book: &Book, author_name: &str) -> Result<()>;
    async fn delete_book(&self, book_id: &str) -> Result<()>;
    async fn upsert_review(&self, review: &Review) -> Result<()>;
    async fn delete_review(&self, review_id: &str) -> Result<()>;
    async fn search(&self, q: &str, limit: usize) -> Result<Vec<SearchHit>>;
}