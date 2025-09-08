use rocket::{Route, State};
use rocket::form::{Form, FromForm};           // Para manejar <form> (UI)
use rocket_dyn_templates::Template;            // Para renderizar Tera
use rocket::response::Redirect;
use std::collections::HashMap;
use serde::Serialize;                          // Para serializar structs hacia la vista

use futures_util::TryStreamExt;                // Para iterar cursores async de Mongo
use mongodb::{
    bson::{doc, oid::ObjectId},
    Collection,                                // Tipo de colección tipada
};

use crate::db::AppState;                       // Estado global (contiene Database)
//use crate::models::Author;                     // Modelo de dominio (serde + bson)

// Helper: obtener un handle tipado a la colección "authors".
// - No hace I/O aún; solo devuelve un objeto para luego invocar find/insert/update...
// - El tipo genérico <Author> hace (de)serialización BSON<->Rust automática con serde.
fn col(state: &State<AppState>) -> Collection<Author> {
    state.db.collection::<Author>("authors")
}

use crate::models::{Author, Book, Review, Sale};   // importar todos los modelos

/* ====== UI ===== */

// Estructura del <form> de creación en la vista (solo campos que el usuario rellena).
// FromForm habilita que Rocket parsee application/x-www-form-urlencoded.
#[derive(FromForm)]
pub struct AuthorForm {
    pub name: String,
    pub country: Option<String>,
    pub description: Option<String>,
    pub date_of_birth: Option<String>,
}

// "Proyección" para la vista: evitamos exponer ObjectId y lo convertimos a String.
// Serialize permite pasar este struct directo al contexto Tera.
#[derive(Serialize)]
struct AuthorView {
    id: String,
    name: String,
    country: Option<String>,
    description: Option<String>,
    date_of_birth: Option<String>,
}

// Contexto que enviamos al template de índice.
// - authors: la lista renderizable
// - q: el query de búsqueda (para rellenar el input)
// - message: mensajes flash/banners (si los usáramos)
#[derive(Serialize)]
struct AuthorsCtx {
    authors: Vec<AuthorView>,
    q: Option<String>,
    message: Option<String>,
    editing: Option<AuthorView>,
}

#[derive(Serialize)]
struct AuthorCtx {
    author: AuthorView,
}

// GET /authors?q=
// Renderiza la vista con listado + formulario de creación.
// Notas:
// - Convertimos Author -> AuthorView (id a hex para URLs).
// - Template::render("authors/index", &ctx) busca templates/authors/index.html.tera
#[get("/?<q>")]
pub async fn index(state: &State<AppState>, q: Option<String>) -> Template {
    let c = col(state);

    let filter = if let Some(ref s) = q {
        doc! { "name": { "$regex": s, "$options": "i" } }
    } else {
        doc! {}
    };

    let mut cur = c.find(filter).await.expect("find authors");
    let mut authors = Vec::<AuthorView>::new();
    while let Some(a) = cur.try_next().await.expect("cursor") {
        if let Some(id) = a.id {
            authors.push(AuthorView {
                id: id.to_hex(),          // ObjectId -> String amigable para URLs
                name: a.name,
                country: a.country,
                description: a.description,
                date_of_birth: a.date_of_birth,
            });
        }
    }

    Template::render("authors/index", &AuthorsCtx { authors, q, message: None, editing: None })
}

// POST /authors/create
// Crea un autor desde el formulario de la vista y vuelve a renderizar el índice.
// Notas:
// - Aquí ignoramos el error de insert por simplicidad (en prod, manejarlo).
// - Podríamos hacer Redirect::to("/authors/ui") si preferimos PRG pattern.
#[post("/create", data = "<form>")]
pub async fn create(state: &State<AppState>, form: Form<AuthorForm>) -> Template {
    let c = col(state);
    let f = form.into_inner();

    let a = Author {
        id: None,
        name: f.name,
        date_of_birth: f.date_of_birth,
        country: f.country,
        description: f.description,
        image_path: None,  // Default to None for new authors
    };

    let _ = c.insert_one(&a).await;
    // Invalidate caches affected by author creation
    state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;
    state.cache_del_pref("search:books:").await;
    // Re-render directo del índice (simple y efectivo)
    index(state, None).await
}

// POST /authors/delete/<id>
// Borra un autor desde la tabla de la vista y re-renderiza la lista.
// Notas:
// - Si el id no es válido o no existe, simplemente recargamos.
// - Igual que arriba, podríamos usar Redirect::to("/authors/ui").
#[post("/delete/<id>")]
pub async fn delete_author(state: &State<AppState>, id: &str) -> Template {
    let db = &state.db;

    if let Ok(author_id) = ObjectId::parse_str(id) {
        let authors = db.collection::<Author>("authors");
        let books = db.collection::<Book>("books");
        let reviews = db.collection::<Review>("reviews");
        let sales = db.collection::<Sale>("sales");

        // buscar libros del autor
        if let Ok(mut cursor) = books.find(doc! { "author_id": &author_id }).await {
            while let Some(book) = cursor.try_next().await.unwrap_or(None) {
                if let Some(book_id) = book.id {
                    // borrar reseñas y ventas de cada libro
                    let _ = reviews.delete_many(doc! { "book_id": &book_id }).await;
                    let _ = sales.delete_many(doc! { "book_id": &book_id }).await;

                    // Invalidate per-book cached average score
                    let avg_key = format!("book:{}:avg_score", book_id.to_hex());
                    state.cache_del_key(&avg_key).await;
                }
            }
        }

        // borrar libros del autor
        let _ = books.delete_many(doc! { "author_id": &author_id }).await;

        // borrar autor
        let _ = authors.delete_one(doc! { "_id": &author_id }).await;

        // Invalidate caches affected by author deletion
        state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;
        state.cache_del_pref("search:books:").await;
    }

    index(state, None).await
}


// GET /authors/edit/<id>
// Carga el autor a editar y renderiza la vista dedicada de edición.
#[get("/edit/<id>")]
pub async fn edit(state: &State<AppState>, id: &str) -> Template {
    let c = col(state);
    if let Ok(oid) = ObjectId::parse_str(id) {
        if let Ok(Some(a)) = c.find_one(doc! {"_id": oid}).await {
            if let Some(oid) = a.id {
                let view = AuthorView {
                    id: oid.to_hex(),
                    name: a.name,
                    country: a.country,
                    description: a.description,
                    date_of_birth: a.date_of_birth,
                };
                return Template::render("authors/edit", &AuthorCtx { author: view });
            }
        }
    }
    // Si no se encuentra o hay error, volvemos al índice
    index(state, None).await
}

// POST /authors/update/<id>
// Actualiza y redirige al listado (PRG pattern).
#[post("/update/<id>", data = "<form>")]
pub async fn update(state: &State<AppState>, id: &str, form: Form<AuthorForm>) -> Redirect {
    let c = col(state);
    if let Ok(oid) = ObjectId::parse_str(id) {
        let f = form.into_inner();
        let mut set_doc = doc! { "name": f.name };
        if let Some(country) = f.country { set_doc.insert("country", country); }
        if let Some(desc) = f.description { set_doc.insert("description", desc); }
        if let Some(dob_str) = f.date_of_birth {
            set_doc.insert("date_of_birth", dob_str);
        }
        let _ = c.find_one_and_update(doc! {"_id": oid}, doc! {"$set": set_doc}).await;
        // Invalidate caches affected by author update
        state.cache_del_key(AppState::AUTHORS_SUMMARY_CACHE_KEY).await;
        state.cache_del_pref("search:books:").await;
    }
    Redirect::to("/authors")
}

// GET /authors/create
// Renderiza la página con el formulario de creación (usa el parcial _form).
#[get("/create")]
pub async fn create_page() -> Template {
    let ctx: HashMap<&str, &str> = HashMap::new(); // Tera requiere objeto (no unit)
    Template::render("authors/create", &ctx)
}

// GET /authors/read/<id>
// Renderiza la vista de solo lectura con los datos del autor.
#[get("/read/<id>")]
pub async fn read(state: &State<AppState>, id: &str) -> Template {
    let c = col(state);
    if let Ok(oid) = ObjectId::parse_str(id) {
        if let Ok(Some(a)) = c.find_one(doc! {"_id": oid}).await {
            if let Some(oid) = a.id {
                let view = AuthorView {
                    id: oid.to_hex(),
                    name: a.name,
                    country: a.country,
                    description: a.description,
                    date_of_birth: a.date_of_birth,
                };
                return Template::render("authors/read", &AuthorCtx { author: view });
            }
        }
    }
    // Si no se encuentra o hay error, volvemos al índice
    index(state, None).await
}

// Registro de rutas SOLO UI para montar en main.rs
pub fn routes() -> Vec<Route> {
    routes![index, create_page, create, delete_author, edit, update, read]
}