use rocket::{Route, State};
use rocket::form::{Form, FromForm};
use rocket_dyn_templates::Template;
use rocket::response::Redirect;
use serde::Serialize;
use futures_util::TryStreamExt;
use std::collections::HashMap;

use crate::models::{Review, Sale};   // importar todos los modelos
use crate::routes::reviews::reviews_col;          // helper público para obtener reviews
use crate::routes::sales::sales_col;              // helper público para obtener sales

use mongodb::{
    bson::{doc, oid::ObjectId},
    Collection,
};

use crate::db::AppState;
use crate::models::{Author, Book};

pub fn books_col(state: &State<AppState>) -> Collection<Book> {
    state.db.collection::<Book>("books")
}
pub fn authors_col(state: &State<AppState>) -> Collection<Author> {
    state.db.collection::<Author>("authors")
}


#[derive(FromForm)]
pub struct BookForm {
    pub title: String,
    pub author_id: String,               
    pub summary: Option<String>,
    pub publication_date: Option<String>,
    pub total_sales: Option<i64> 
}

#[derive(Serialize)]
struct BookView {
    id: String,
    author_id: String,
    author_name: String,                 
    title: String,
    summary: Option<String>,
    publication_date: Option<String>,
    pub total_sales: Option<i64> 
}

#[derive(Serialize)]
struct AuthorOpt {
    id: String,
    name: String,
}


#[derive(Serialize)]
struct BooksCtx {
    books: Vec<BookView>,
    authors: Vec<AuthorOpt>,            // para el <select> en create/edit
    q: Option<String>,                  // búsqueda por título
    message: Option<String>,
}

#[derive(Serialize)]
struct BookCtx {
    book: BookView,
    authors: Vec<AuthorOpt>,
}

/* ===== Handlers ===== */

// GET /books?q=
#[get("/?<q>")]
pub async fn index(state: &State<AppState>, q: Option<String>) -> Template {
    let books_c = books_col(state);
    let authors_c = authors_col(state);

    // Traer autores para mapear nombres
    let mut a_cur = authors_c.find(doc!{}).await.expect("authors find");
    let mut authors = Vec::<AuthorOpt>::new();
    let mut author_name_by_id = HashMap::<String, String>::new();
    while let Some(a) = a_cur.try_next().await.expect("cursor authors") {
        if let Some(id) = a.id {
            let s = id.to_hex();
            let name = a.name.clone();
            author_name_by_id.insert(s.clone(), name.clone());
            authors.push(AuthorOpt { id: s, name });
        }
    }

    // filtro por búsqueda en título (regex simple; ya tienes índice text para luego)
    let filter = if let Some(ref s) = q {
        doc! { "title": { "$regex": s, "$options": "i" } }
    } else { doc!{} };

    let mut b_cur = books_c.find(filter).await.expect("books find");
    let mut books = Vec::<BookView>::new();
    loop {
        match b_cur.try_next().await {
            Ok(Some(b)) => {
                if let Some(id) = b.id {
                    let aid = b.author_id.to_hex();
                    let author_name = author_name_by_id
                        .get(&aid)
                        .cloned()
                        .unwrap_or_else(|| "(unknown)".into());
                    books.push(BookView {
                        id: id.to_hex(),
                        author_id: aid,
                        author_name,
                        title: b.title,
                        summary: b.summary,
                        publication_date: b.publication_date,
                        total_sales: b.total_sales,
                    });
                }
            }
            Ok(None) => break,
            Err(e) => { eprintln!("books cursor: {e}"); break; }
        }
    }

    Template::render("books/index", &BooksCtx { books, authors, q, message: None })
}

// GET /books/create
#[get("/create")]
pub async fn create_page(state: &State<AppState>) -> Template {
    let authors_c = authors_col(state);
    let mut a_cur = authors_c.find(doc!{}).await.expect("authors find");
    let mut authors = Vec::<AuthorOpt>::new();
    while let Some(a) = a_cur.try_next().await.expect("cursor authors") {
        if let Some(id) = a.id {
            authors.push(AuthorOpt { id: id.to_hex(), name: a.name });
        }
    }

    Template::render("books/create", &serde_json::json!({ "authors": authors }))
}

// POST /books/create
#[post("/create", data = "<form>")]
pub async fn create(state: &State<AppState>, form: Form<BookForm>) -> Redirect {
    let books_c = books_col(state);
    let authors_c = authors_col(state);
    let f = form.into_inner();

    // validar ObjectId y existencia del autor
    let author_oid = match ObjectId::parse_str(&f.author_id) {
        Ok(oid) => oid,
        Err(_) => return Redirect::to("/books"),
    };
    let exists = authors_c.find_one(doc!{"_id": &author_oid}).await.ok().flatten().is_some();
    if !exists {
        return Redirect::to("/books");
    }

    let b = Book {
        id: None,
        author_id: author_oid,
        title: f.title,
        summary: f.summary,
        publication_date: f.publication_date,
        total_sales: None,
    };
    let _ = books_c.insert_one(&b).await;
    // invalidate caches: authors summary and search results
    state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;
    state.cache_del_pref("search:books:").await;
    Redirect::to("/books")
}

// GET /books/edit/<id>
#[get("/edit/<id>")]
pub async fn edit(state: &State<AppState>, id: &str) -> Template {
    let books_c = books_col(state);
    let authors_c = authors_col(state);

    // cargar libro
    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_) => return index(state, None).await };
    let book = match books_c.find_one(doc!{"_id": oid}).await { Ok(x)=>x, Err(_) => None };
    if book.is_none() { return index(state, None).await; }
    let book = book.unwrap();

    // cargar autores
    let mut a_cur = authors_c.find(doc!{}).await.expect("authors find");
    let mut authors = Vec::<AuthorOpt>::new();
    let mut author_name_by_id = HashMap::<String, String>::new();
    while let Some(a) = a_cur.try_next().await.expect("cursor authors") {
        if let Some(aid) = a.id {
            let s = aid.to_hex();
            author_name_by_id.insert(s.clone(), a.name.clone());
            authors.push(AuthorOpt { id: s, name: a.name });
        }
    }

    // armar BookView
    let id_s = book.id.unwrap().to_hex();
    let aid_s = book.author_id.to_hex();
    let author_name = author_name_by_id.get(&aid_s).cloned().unwrap_or_else(|| "(unknown)".into());

    let view = BookView {
        id: id_s,
        author_id: aid_s,
        author_name,
        title: book.title,
        summary: book.summary,
        publication_date: book.publication_date,
        total_sales: book.total_sales,
    };

    Template::render("books/edit", &BookCtx { book: view, authors })
}

// POST /books/update/<id>
#[post("/update/<id>", data = "<form>")]
pub async fn update(state: &State<AppState>, id: &str, form: Form<BookForm>) -> Redirect {
    let books_c = books_col(state);
    let authors_c = authors_col(state);
    let f = form.into_inner();

    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_) => return Redirect::to("/books") };
    let author_oid = match ObjectId::parse_str(&f.author_id) { Ok(x)=>x, Err(_) => return Redirect::to("/books") };

    // validar existencia del autor
    let exists = authors_c.find_one(doc!{"_id": &author_oid}).await.ok().flatten().is_some();
    if !exists { return Redirect::to("/books"); }

    let mut set_doc = doc! {
        "title": f.title,
        "author_id": author_oid
    };
    if let Some(s) = f.summary { set_doc.insert("summary", s); }
    if let Some(p) = f.publication_date { set_doc.insert("publication_date", p); }

    let _ = books_c.find_one_and_update(doc!{"_id": oid}, doc!{"$set": set_doc}).await;
    // invalidate caches: authors summary and search results
    state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;
    state.cache_del_pref("search:books:").await;
    Redirect::to("/books")
}

// POST /books/delete/<id>
// #[post("/delete/<id>")]
#[post("/delete/<id>")]
pub async fn delete(state: &State<AppState>, id: &str) -> Redirect {
    let books_c = books_col(state);
    let reviews_c = reviews_col(state);
    let sales_c = sales_col(state);

    if let Ok(book_id) = ObjectId::parse_str(id) {
        // eliminar reseñas asociadas
        let _ = reviews_c.delete_many(doc! {"book_id": &book_id}).await;

        // eliminar ventas asociadas
        let _ = sales_c.delete_many(doc! {"book_id": &book_id}).await;

        // eliminar el libro
        let _ = books_c.delete_one(doc! {"_id": &book_id}).await;

        // invalidate caches related to this book
        state.cache_del_key(&AppState::key_book_avg(&book_id.to_hex())).await;
        state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;
        state.cache_del_pref("search:books:").await;
    }

    Redirect::to("/books")
}

// GET /books/read/<id>
#[get("/read/<id>")]
pub async fn read(state: &State<AppState>, id: &str) -> Template {
    let books_c = books_col(state);
    let authors_c = authors_col(state);

    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_) => return index(state, None).await };
    let book = match books_c.find_one(doc!{"_id": oid}).await { Ok(x)=>x, Err(_) => None };
    if book.is_none() { return index(state, None).await; }
    let b = book.unwrap();

    // nombre del autor
    let aid_s = b.author_id.to_hex();
    let author_name = authors_c
        .find_one(doc!{"_id": &b.author_id}).await.ok().flatten().map(|a| a.name).unwrap_or_else(|| "(unknown)".into());

    let view = BookView {
        id: b.id.unwrap().to_hex(),
        author_id: aid_s,
        author_name,
        title: b.title,
        summary: b.summary,
        publication_date: b.publication_date,
        total_sales: b.total_sales,
    };

    Template::render("books/read", &BookCtx { book: view, authors: vec![] })
}

pub fn routes() -> Vec<Route> {
    routes![index, create_page, create, edit, update, delete, read]
}