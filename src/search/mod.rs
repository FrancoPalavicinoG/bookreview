pub struct SearchHit {
    pub id: String,     // ej: book_id o review_id
    pub score: f32,
}

pub trait SearchEngine: Send + Sync {
    // Indexación mínima que luego conectaremos a ES/OpenSearch
    fn index_book(&self, _book_id: &str, _title: &str, _summary: &str) {}
    fn delete_book(&self, _book_id: &str) {}

    fn index_review(&self, _review_id: &str, _book_id: &str, _content: &str, _score: i32) {}
    fn delete_review(&self, _review_id: &str) {}

    // Búsqueda por texto (libros por ahora)
    fn search_books(&self, _q: &str, _limit: usize) -> Vec<SearchHit> { vec![] }
}

// No-op: no indexa ni devuelve resultados
pub struct NoopSearch;
impl SearchEngine for NoopSearch {}