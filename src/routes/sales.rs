use rocket::{Route, State};
use rocket::form::{Form, FromForm};
use rocket::response::Redirect;
use rocket_dyn_templates::Template;
use serde::Serialize;
use futures_util::TryStreamExt;
use std::collections::HashMap;

use mongodb::{
    bson::{doc, oid::ObjectId, Bson, Document},
    Collection,
};

use crate::db::AppState;
use crate::models::{Book, Sale};

/* ========= Helpers de colecciones ========= */

pub fn sales_col(state: &State<AppState>) -> Collection<Sale> {
    state.db.collection::<Sale>("sales")
}
pub fn books_col(state: &State<AppState>) -> Collection<Book> {
    state.db.collection::<Book>("books")
}

/* ========= Formularios y vistas ========= */

#[derive(FromForm)]
pub struct SaleForm {
    pub book_id: String,    // del <select>
    pub year: i32,
    pub units: i64,
}

#[derive(Serialize)]
struct BookOpt {
    id: String,
    title: String,
}

#[derive(Serialize)]
struct SaleView {
    id: String,
    book_id: String,
    book_title: String,
    year: i32,
    units: i64,
}

#[derive(Serialize)]
struct SalesCtx {
    sales: Vec<SaleView>,
    books: Vec<BookOpt>,
    q_book: Option<String>, // filtro por book_id
    q_year: Option<i32>,    // filtro por year
    message: Option<String>,
}

#[derive(Serialize)]
struct SaleCtx {
    sale: SaleView,
    books: Vec<BookOpt>,
}

/* ========= Recalcular total_sales del Book ========= */

async fn recompute_book_total(state: &State<AppState>, book_oid: &ObjectId) {
    let sales = sales_col(state);
    let books = books_col(state);

    // pipeline: sum(units) por book_id
    let pipeline = vec![
        doc! { "$match": { "book_id": book_oid } },
        doc! { "$group": { "_id": Bson::Null, "total": { "$sum": "$units" } } },
    ];

    let mut cur = match sales.aggregate(pipeline).await {
        Ok(c) => c,
        Err(e) => { eprintln!("aggregate sales error: {e}"); return; }
    };

    let mut total: i64 = 0;
    if let Ok(Some(d)) = cur.try_next().await {
        if let Ok(t) = d.get_i64("total") { total = t; }
        else if let Some(Bson::Int64(v)) = d.get("total") { total = *v; }
        else if let Some(Bson::Int32(v)) = d.get("total") { total = *v as i64; }
    }

    // actualizar books.total_sales
    let _ = books
        .find_one_and_update(
            doc! { "_id": book_oid },
            doc! { "$set": { "total_sales": total } },
        )
        .await;
}


// GET /sales?q_book=<book_id>&q_year=<year>
#[get("/?<q_book>&<q_year>")]
pub async fn index(
    state: &State<AppState>,
    q_book: Option<String>,
    q_year: Option<i32>,
) -> Template {
    let s_c = sales_col(state);
    let b_c = books_col(state);

    // map de book_id -> title y opciones para select
    let mut b_cur = b_c.find(doc!{}).await.expect("books find");
    let mut books = Vec::<BookOpt>::new();
    let mut title_by_id = HashMap::<String, String>::new();
    while let Ok(Some(b)) = b_cur.try_next().await {
        if let Some(id) = b.id {
            let s = id.to_hex();
            title_by_id.insert(s.clone(), b.title.clone());
            books.push(BookOpt { id: s, title: b.title });
        }
    }

    // filtros
    let mut filter = Document::new();
    if let Some(ref bid) = q_book {
        if let Ok(oid) = ObjectId::parse_str(bid) {
            filter.insert("book_id", oid);
        }
    }
    if let Some(y) = q_year {
        filter.insert("year", y);
    }

    // query
    let mut sales = Vec::<SaleView>::new();
    let mut cur = s_c.find(filter).await.expect("sales find");
    loop {
        match cur.try_next().await {
            Ok(Some(s)) => {
                if let Some(id) = s.id {
                    let bid = s.book_id.to_hex();
                    let book_title = title_by_id.get(&bid).cloned().unwrap_or_else(|| "(unknown)".into());
                    sales.push(SaleView {
                        id: id.to_hex(),
                        book_id: bid,
                        book_title,
                        year: s.year,
                        units: s.units,
                    });
                }
            }
            Ok(None) => break,
            Err(e) => { eprintln!("sales cursor: {e}"); break; }
        }
    }

    Template::render(
        "sales/index",
        &SalesCtx { sales, books, q_book, q_year, message: None }
    )
}

// GET /sales/create
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
    Template::render("sales/create", &serde_json::json!({ "books": books }))
}

// POST /sales/create
// si ya existe (book_id, year), actualizamos units (replace) en lugar de insertar duplicado.
#[post("/create", data = "<form>")]
pub async fn create(state: &State<AppState>, form: Form<SaleForm>) -> Redirect {
    let s_c = sales_col(state);
    let b_c = books_col(state);
    let f = form.into_inner();

    let book_oid = match ObjectId::parse_str(&f.book_id) { Ok(x)=>x, Err(_)=> return Redirect::to("/sales") };
    // validar libro
    let exists = b_c.find_one(doc!{"_id": &book_oid}).await.ok().flatten().is_some();
    if !exists { return Redirect::to("/sales"); }

    // si existe la venta de ese año, actualizamos; si no, insertamos
    let key = doc! { "book_id": &book_oid, "year": f.year };
    let found = s_c.find_one(key.clone()).await.ok().flatten();
    if found.is_some() {
        let _ = s_c
            .find_one_and_update(key, doc!{ "$set": { "units": f.units } })
            .await;
    } else {
        let s = Sale { id: None, book_id: book_oid, year: f.year, units: f.units };
        let _ = s_c.insert_one(&s).await;
    }

    // recomputar total del libro
    recompute_book_total(state, &book_oid).await;

    Redirect::to("/sales")
}

// GET /sales/edit/<id>
#[get("/edit/<id>")]
pub async fn edit(state: &State<AppState>, id: &str) -> Template {
    let s_c = sales_col(state);
    let b_c = books_col(state);

    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_)=> return index(state, None, None).await };
    let s = match s_c.find_one(doc!{"_id": oid}).await { Ok(x)=>x, Err(_)=> None };
    if s.is_none() { return index(state, None, None).await; }
    let s = s.unwrap();

    // cargar libros para select (permitimos cambiar libro; si no lo quieres, puedes hacerlo readonly)
    let mut books = Vec::<BookOpt>::new();
    let mut cur = b_c.find(doc!{}).await.expect("books find");
    while let Ok(Some(b)) = cur.try_next().await {
        if let Some(id) = b.id {
            books.push(BookOpt { id: id.to_hex(), title: b.title });
        }
    }

    let book_title = b_c
        .find_one(doc!{"_id": &s.book_id}).await.ok().flatten().map(|b| b.title).unwrap_or_else(|| "(unknown)".into());

    let view = SaleView {
        id: s.id.unwrap().to_hex(),
        book_id: s.book_id.to_hex(),
        book_title,
        year: s.year,
        units: s.units,
    };

    Template::render("sales/edit", &SaleCtx { sale: view, books })
}

// POST /sales/update/<id>
#[post("/update/<id>", data = "<form>")]
pub async fn update(state: &State<AppState>, id: &str, form: Form<SaleForm>) -> Redirect {
    let s_c = sales_col(state);
    let b_c = books_col(state);
    let f = form.into_inner();

    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_)=> return Redirect::to("/sales") };
    let new_book = match ObjectId::parse_str(&f.book_id) { Ok(x)=>x, Err(_)=> return Redirect::to("/sales") };

    // validar libro
    let exists = b_c.find_one(doc!{"_id": &new_book}).await.ok().flatten().is_some();
    if !exists { return Redirect::to("/sales"); }

    // necesitamos saber el book_id anterior para recomputar si cambia
    let old = s_c.find_one(doc!{"_id": oid}).await.ok().flatten();

    // si cambió (book_id, year) hay que respetar la unicidad (book_id, year)
    // estrategia simple: si ya existe destino, actualizamos su units; y eliminamos el doc original.
    if let Some(prev) = old {
        if prev.book_id != new_book || prev.year != f.year {
            // fusionar (upsert manual)
            let key = doc!{"book_id": &new_book, "year": f.year};
            let found = s_c.find_one(key.clone()).await.ok().flatten();
            if found.is_some() {
                let _ = s_c.find_one_and_update(key, doc!{"$set": {"units": f.units}}).await;
            } else {
                let _ = s_c.insert_one(&Sale { id: None, book_id: new_book, year: f.year, units: f.units }).await;
            }
            let _ = s_c.delete_one(doc!{"_id": oid}).await;

            // recomputar ambos libros (origen y destino)
            recompute_book_total(state, &prev.book_id).await;
            recompute_book_total(state, &new_book).await;

            return Redirect::to("/sales");
        }
    }

    // misma clave, solo actualizamos units
    let _ = s_c
        .find_one_and_update(
            doc!{"_id": oid},
            doc!{"$set": {"book_id": &new_book, "year": f.year, "units": f.units}}
        )
        .await;

    // recomputar libro
    recompute_book_total(state, &new_book).await;

    Redirect::to("/sales")
}

// POST /sales/delete/<id>
#[post("/delete/<id>")]
pub async fn delete(state: &State<AppState>, id: &str) -> Redirect {
    let s_c = sales_col(state);

    // obtener book_id para recomputar luego
    if let Ok(oid) = ObjectId::parse_str(id) {
        if let Ok(Some(prev)) = s_c.find_one(doc!{"_id": oid}).await {
            let book = prev.book_id.clone();
            let _ = s_c.delete_one(doc!{"_id": oid}).await;
            recompute_book_total(state, &book).await;
            return Redirect::to("/sales");
        }
    }
    Redirect::to("/sales")
}

// GET /sales/read/<id>
#[get("/read/<id>")]
pub async fn read(state: &State<AppState>, id: &str) -> Template {
    let s_c = sales_col(state);
    let b_c = books_col(state);

    let oid = match ObjectId::parse_str(id) { Ok(x)=>x, Err(_)=> return index(state, None, None).await };
    let s = match s_c.find_one(doc!{"_id": oid}).await { Ok(x)=>x, Err(_)=> None };
    if s.is_none() { return index(state, None, None).await; }
    let s = s.unwrap();

    let book_title = b_c
        .find_one(doc!{"_id": &s.book_id}).await.ok().flatten().map(|b| b.title).unwrap_or_else(|| "(unknown)".into());

    let view = SaleView {
        id: s.id.unwrap().to_hex(),
        book_id: s.book_id.to_hex(),
        book_title,
        year: s.year,
        units: s.units,
    };

    Template::render("sales/read", &SaleCtx { sale: view, books: vec![] })
}

pub fn routes() -> Vec<Route> {
    routes![index, create_page, create, edit, update, delete, read]
}