use std::sync::Arc;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use opensearch::{OpenSearch, http::transport::Transport, indices::IndicesCreateParts};
use opensearch::http::response::Response;
use opensearch::SearchParts;
use serde_json::json;

use crate::models::{Book, Review, BookSearchDoc, ReviewSearchDoc};
use super::{SearchEngine, SearchHit};

pub struct OpenSearchEngine {
    client: OpenSearch,
}

impl OpenSearchEngine {
    pub async fn new(url: &str) -> Result<Self> {
        let transport = Transport::single_node(url)?;
        let client = OpenSearch::new(transport);

        // Crear índice "books" si no existe
        match client.indices()
            .create(IndicesCreateParts::Index("books"))
            .body(json!({
                "mappings": {
                    "properties": {
                        "title": { "type": "text" },
                        "summary": { "type": "text" },
                        "author_name": { "type": "text" },
                        "publication_date": { "type": "date" }
                    }
                }
            }))
            .send()
            .await
        {
            Ok(_) => {}
            Err(err) => {
                if !err.to_string().contains("resource_already_exists_exception") {
                    return Err(err.into());
                }
            }
        }

        // Crear índice "reviews" si no existe
        match client.indices()
            .create(IndicesCreateParts::Index("reviews"))
            .body(json!({
                "mappings": {
                    "properties": {
                        "text": { "type": "text" },
                        "score": { "type": "integer" },
                        "book_id": { "type": "keyword" }
                    }
                }
            }))
            .send()
            .await
        {
            Ok(_) => {}
            Err(err) => {
                if !err.to_string().contains("resource_already_exists_exception") {
                    return Err(err.into());
                }
            }
        }

        Ok(Self { client })
    }
}

#[async_trait]
impl SearchEngine for OpenSearchEngine {
    async fn upsert_book(&self, book: &Book, author_name: &str) -> Result<()> {
        let book_id = book.id.as_ref().ok_or_else(|| anyhow!("Book id missing"))?.to_hex();

        let doc = BookSearchDoc {
            book_id: book_id.clone(),
            title: book.title.clone(),
            summary: book.summary.clone(),
            author_name: author_name.to_string(),
            publication_date: book.publication_date.clone(),
        };

        self.client.index(opensearch::IndexParts::IndexId("books", &book_id))
            .body(&doc)
            .send()
            .await?;

        Ok(())
    }

    async fn delete_book(&self, book_id: &str) -> Result<()> {
        self.client.delete(opensearch::DeleteParts::IndexId("books", book_id))
            .send()
            .await?;
        Ok(())
    }

    async fn upsert_review(&self, review: &Review) -> Result<()> {
        let review_id = review.id.as_ref().ok_or_else(|| anyhow!("Review id missing"))?.to_hex();
        let book_id = review.book_id.to_hex();

        let doc = ReviewSearchDoc {
            review_id: review_id.clone(),
            book_id,
            text: review.text.clone(),
            score: review.score,
        };

        self.client.index(opensearch::IndexParts::IndexId("reviews", &review_id))
            .body(&doc)
            .send()
            .await?;

        Ok(())
    }

    async fn delete_review(&self, review_id: &str) -> Result<()> {
        self.client.delete(opensearch::DeleteParts::IndexId("reviews", review_id))
            .send()
            .await?;
        Ok(())
    }

    async fn search(&self, q: &str, limit: usize) -> Result<Vec<SearchHit>> {
        let res = self.client
            .search(SearchParts::Index(&["books", "reviews"]))
            .body(json!({
                "query": {
                    "multi_match": {
                        "query": q,
                        "fields": ["title^2", "summary", "text"]
                    }
                },
                "size": limit
            }))
            .send()
            .await?;

        let json: serde_json::Value = res.json().await?;
        let mut hits = Vec::new();

        if let Some(arr) = json["hits"]["hits"].as_array() {
            for h in arr {
                hits.push(SearchHit {
                    id: h["_id"].as_str().unwrap_or("").to_string(),
                    kind: h["_index"].as_str().unwrap_or("").to_string(),
                    score: h["_score"].as_f64().unwrap_or(0.0) as f32,
                    highlight: h.get("highlight").map(|v| v.to_string()),
                });
            }
        }

        Ok(hits)
    }
}