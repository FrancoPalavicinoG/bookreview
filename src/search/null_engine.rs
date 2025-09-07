use anyhow::Result;
use async_trait::async_trait;

use crate::models::{Book, Review};
use super::{SearchEngine, SearchHit};

pub struct NullSearchEngine;

#[async_trait]
impl SearchEngine for NullSearchEngine {
    async fn upsert_book(&self, _: &Book, _: &str) -> Result<()> { Ok(()) }
    async fn delete_book(&self, _: &str) -> Result<()> { Ok(()) }
    async fn upsert_review(&self, _: &Review) -> Result<()> { Ok(()) }
    async fn delete_review(&self, _: &str) -> Result<()> { Ok(()) }
    async fn search(&self, _: &str, _: usize) -> Result<Vec<SearchHit>> { Ok(vec![]) }
}