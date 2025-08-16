use std::collections::HashSet;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AuthorDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    date_of_birth: Option<String>,
    country: Option<String>,
    description: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let uri = std::env::var("MONGO_URI").expect("MONGO_URI not set");
    let db_name = std::env::var("DB_NAME").expect("DB_NAME not set");

    let mut client_opts = ClientOptions::parse(&uri).await?;
    client_opts.app_name = Some("bookreview-seeder".into());
    let client = Client::with_options(client_opts)?;
    let db = client.database(&db_name);
    let authors: Collection<AuthorDoc> = db.collection("authors");

    // Countries (ISO-2 codes)
    let countries = [
        "US","UK","CA","AR","CL","MX","BR","CO","ES","FR","DE","IT","JP","KR","CN","IN","NG","ZA","SE","NO",
    ];

    let mut rng = rand::thread_rng();
    let mut uniques = HashSet::<String>::new();
    let mut docs: Vec<AuthorDoc> = Vec::new();

    while docs.len() < 50 {
        // Name (EN locale only)
        let name: String = Name(EN).fake();

        if !uniques.insert(name.clone()) {
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

        docs.push(AuthorDoc {
            id: None,
            name,
            date_of_birth: dob,
            country,
            description: Some(description),
        });
    }

    // Wipe and seed (API v3: no Options)
    authors.delete_many(doc! {}).await?;
    let res = authors.insert_many(docs).await?;
    println!("Seeded authors: {}", res.inserted_ids.len());

    Ok(())
}