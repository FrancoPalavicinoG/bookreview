use std::time::Duration;
use async_trait::async_trait;

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, _key: &str) -> Option<Vec<u8>> { None }
    async fn set(&self, _key: &str, _value: &[u8], _ttl: Option<Duration>) {}
    async fn del(&self, _key: &str) {}
    async fn del_prefix(&self, _prefix: &str) {}
}

// No-op: no cachea nada
pub struct NoopCache;

#[async_trait]
impl Cache for NoopCache {}

#[cfg(feature = "redis-cache")]
pub mod redis;