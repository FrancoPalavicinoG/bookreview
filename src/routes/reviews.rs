use rocket::{Route, State};
use rocket::form::{Form, FromForm};
use rocket::response::Redirect;
use rocket_dyn_templates::Template;
use serde::Serialize;
use futures_util::TryStreamExt;
use std::collections::HashMap;

use mongodb::{
    bson::{doc, oid::ObjectId},
    Collection,
};

use crate::db::AppState;
use crate::models::{Book, Review};

/* ========= Helpers de colecciones ========= */

pub fn reviews_col(state: &State<AppState>) -> Collection<Review> {
    state.db.collection::<Review>("reviews")
}
pub fn books_col(state: &State<AppState>) -> Collection<Book> {
    state.db.collection::<Book>("books")
}

/* ===== Formularios y vistas ===== */

#[derive(FromForm)]
pub struct ReviewForm {
    pub book_id: String,        // viene del <select>
    pub text: String,
    pub score: i32,             // 1..5
    pub up_votes: Option<i64>,  // default 0
}

#[derive(Serialize)]
struct BookOpt {
    id: String,
    title: String,
}

#[derive(Serialize)]
struct ReviewView {
    id: String,
    book_id: String,
    book_title: String,
    text: String,
    score: i32,
    up_votes: i64,
}

#[derive(Serialize)]
struct ReviewsCtx {
    reviews: Vec<ReviewView>,
    books: Vec<BookOpt>,        // para el <select> en create/edit
    q: Option<String>,          // búsqueda por texto
    message: Option<String>,
}

#[derive(Serialize)]
struct ReviewCtx {
    review: ReviewView,
    books: Vec<BookOpt>,
}

// GET /reviews?q=
#[get("/?<q>")]
pub async fn index(state: &State<AppState>, q: Option<String>) -> Template {
    let r_c = reviews_col(state);
    let b_c = books_col(state);

    // Map de book_id -> title
    let mut b_cur = b_c.find(doc!{}).await.expect("books find");
    let mut books = Vec::<BookOpt>::new();
    let mut book_title_by_id = HashMap::<String, String>::new();
    while let Ok(Some(b)) = b_cur.try_next().await {
        if let Some(id) = b.id {
            let s = id.to_hex();
            let title = b.title.clone();
            book_title_by_id.insert(s.clone(), title.clone());
            books.push(BookOpt { id: s, title });
        }
    }

    let filter = if let Some(ref s) = q {
        doc! { "text": { "$regex": s, "$options": "i" } }
    } else { doc!{} };

    let mut rv = Vec::<ReviewView>::new();
    let mut cur = r_c.find(filter).await.expect("reviews find");
    loop {
        match cur.try_next().await {
            Ok(Some(r)) => {
                if let Some(id) = r.id {
                    let bid = r.book_id.to_hex();
                    let book_title = book_title_by_id.get(&bid).cloned().unwrap_or_else(|| "(unknown)".into());
                    rv.push(ReviewView {
                        id: id.to_hex(),
                        book_id: bid,
                        book_title,
                        text: r.text,
                        score: r.score,
                        up_votes: r.up_votes,
                    });
                }
            }
            Ok(None) => break,
            Err(e) => { eprintln!("reviews cursor: {e}"); break; }
        }
    }

    Template::render("reviews/index", &ReviewsCtx { reviews: rv, books, q, message: None })
}

// GET /reviews/create
#[get("/create")]
pub async fn create_page(state: &State<AppState>) -> Template {
    let b_c = books_col(state);
    let mut books = Vec::<BookOpt>::new();
    let mut cur = b_c.find(doc!{}).await.expect("books find");
    while let Ok(Some(b)) = cur.try_next().await {
        if let Some(id) = b.id {
            books.push(BookOpt { id: id.to_hex(), title: b.title });
        }
    }
    Template::render("reviews/create", &serde_json::json!({ "books": books }))
}

// POST /reviews/create
#[post("/create", data = "<form>")]
pub async fn create(state: &State<AppState>, form: Form<ReviewForm>) -> Redirect {
    let r_c = reviews_col(state);
    let b_c = books_col(state);
    let f = form.into_inner();

    // validar book_id y existencia
    let book_oid = match ObjectId::parse_str(&f.book_id) { Ok(x)=>x, Err(_)=> return Redirect::to("/reviews") };
    let exists = b_c.find_one(doc!{"_id": &book_oid}).await.ok().flatten().is_some();
    if !exists { return Redirect::to("/reviews"); }

    // validar score
    let score = f.score.clamp(1, 5);
    let up_votes = f.up_votes.unwrap_or(0).max(0);

    let r = Review {
        id: None,
        book_id: book_oid,
        text: f.text,
        score,
        up_votes,
    };
    let _ = r_c.insert_one(&r).await;

    // invalidate caches affected by this review
    state.cache_del_key(&AppState::key_book_avg(&book_oid.to_hex())).await;
    state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;

    state.cache_del_pref("search:books:").await;

    Redirect::to("/reviews")
}

// GET /reviews/edit/<id>
#[get("/edit/<id>")]
pub async fn edit(state: &State<AppState>, id: &str) -> Template {
    let r_c = reviews_col(state);
    let b_c = books_col(state);

    // cargar review
    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_)=> return index(state, None).await };
    let rev = match r_c.find_one(doc!{"_id": oid}).await { Ok(x)=>x, Err(_)=> None };
    if rev.is_none() { return index(state, None).await; }
    let r = rev.unwrap();

    // cargar libros para select
    let mut books = Vec::<BookOpt>::new();
    let mut cur = b_c.find(doc!{}).await.expect("books find");
    while let Ok(Some(b)) = cur.try_next().await {
        if let Some(id) = b.id {
            books.push(BookOpt { id: id.to_hex(), title: b.title });
        }
    }

    // armar view
    let rv = ReviewView {
        id: r.id.unwrap().to_hex(),
        book_id: r.book_id.to_hex(),
        book_title: b_c
            .find_one(doc!{"_id": &r.book_id}).await.ok().flatten().map(|b| b.title).unwrap_or_else(|| "(unknown)".into()),
        text: r.text,
        score: r.score,
        up_votes: r.up_votes,
    };

    Template::render("reviews/edit", &ReviewCtx { review: rv, books })
}

// POST /reviews/update/<id>
#[post("/update/<id>", data = "<form>")]
pub async fn update(state: &State<AppState>, id: &str, form: Form<ReviewForm>) -> Redirect {
    let r_c = reviews_col(state);
    let b_c = books_col(state);
    let f = form.into_inner();

    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_)=> return Redirect::to("/reviews") };
    let book_oid = match ObjectId::parse_str(&f.book_id) { Ok(x)=>x, Err(_)=> return Redirect::to("/reviews") };

    // validar libro
    let exists = b_c.find_one(doc!{"_id": &book_oid}).await.ok().flatten().is_some();
    if !exists { return Redirect::to("/reviews"); }

    let score = f.score.clamp(1, 5);
    let up_votes = f.up_votes.unwrap_or(0).max(0);

    // load previous review to know previous book_id for cache invalidation
    let prev_review = r_c.find_one(doc!{"_id": oid}).await.ok().flatten();

    let _ = r_c
        .find_one_and_update(
            doc!{"_id": oid},
            doc!{"$set": {
                "book_id": book_oid,
                "text": f.text,
                "score": score,
                "up_votes": up_votes
            }}
        )
        .await;

    // invalidate caches: previous and new book, plus authors summary
    if let Some(prev) = prev_review {
        state.cache_del_key(&AppState::key_book_avg(&prev.book_id.to_hex())).await;
    }
    state.cache_del_key(&AppState::key_book_avg(&book_oid.to_hex())).await;
    state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;

    state.cache_del_pref("search:books:").await;

    Redirect::to("/reviews")
}

// POST /reviews/delete/<id>
#[post("/delete/<id>")]
pub async fn delete(state: &State<AppState>, id: &str) -> Redirect {
    let r_c = reviews_col(state);
    if let Ok(oid) = ObjectId::parse_str(id) {
        // read review to get affected book_id before deleting
        let prev = r_c.find_one(doc!{"_id": oid}).await.ok().flatten();
        let _ = r_c.delete_one(doc!{"_id": oid}).await;

        // invalidate caches
        if let Some(r) = prev {
            state.cache_del_key(&AppState::key_book_avg(&r.book_id.to_hex())).await;
        }
        state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;
        state.cache_del_pref("search:books:").await;
    }
    Redirect::to("/reviews")
}

// GET /reviews/read/<id>
#[get("/read/<id>")]
pub async fn read(state: &State<AppState>, id: &str) -> Template {
    let r_c = reviews_col(state);
    let b_c = books_col(state);

    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_)=> return index(state, None).await };
    let rev = match r_c.find_one(doc!{"_id": oid}).await { Ok(x)=>x, Err(_)=> None };
    if rev.is_none() { return index(state, None).await; }
    let r = rev.unwrap();

    let book_title = b_c
        .find_one(doc!{"_id": &r.book_id}).await.ok().flatten().map(|b| b.title).unwrap_or_else(|| "(unknown)".into());

    let rv = ReviewView {
        id: r.id.unwrap().to_hex(),
        book_id: r.book_id.to_hex(),
        book_title,
        text: r.text,
        score: r.score,
        up_votes: r.up_votes,
    };

    Template::render("reviews/read", &ReviewCtx { review: rv, books: vec![] })
}

pub fn routes() -> Vec<Route> {
    routes![index, create_page, create, edit, update, delete, read]
}