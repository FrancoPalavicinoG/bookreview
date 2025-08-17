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