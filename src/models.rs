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