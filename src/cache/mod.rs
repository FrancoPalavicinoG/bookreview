use std::time::Duration;

pub trait Cache: Send + Sync {
    // genéricas
    fn get(&self, _key: &str) -> Option<Vec<u8>> { None }
    fn set(&self, _key: &str, _value: &[u8], _ttl: Option<Duration>) {}
    fn del(&self, _key: &str) {}
    fn del_prefix(&self, _prefix: &str) {}
}

// No-op: no cachea nada
pub struct NoopCache;
impl Cache for NoopCache {}