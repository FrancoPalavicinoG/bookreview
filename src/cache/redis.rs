use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use redis::{AsyncCommands, aio::ConnectionManager};

use super::Cache;

pub struct RedisCache {
    manager: Arc<ConnectionManager>,
}

impl RedisCache {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        // requiere el feature "connection-manager" en redis = 0.25
        let manager = client.get_connection_manager().await?;
        Ok(Self { manager: Arc::new(manager) })
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut conn = (*self.manager).clone();
        match conn.get::<_, Vec<u8>>(key).await {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) {
        let mut conn = (*self.manager).clone();
        let _: redis::RedisResult<()> = match ttl {
            Some(d) => conn.set_ex(key, value, d.as_secs()).await, // u64 OK
            None => conn.set(key, value).await,
        };
    }

    async fn del(&self, key: &str) {
        let mut conn = (*self.manager).clone();
        let _: redis::RedisResult<()> = conn.del(key).await;
    }

    async fn del_prefix(&self, prefix: &str) {
        let mut conn = (*self.manager).clone();
        let pattern = format!("{}*", prefix);
        let keys_res: redis::RedisResult<Vec<String>> = conn.keys(&pattern).await;
        if let Ok(keys) = keys_res {
            if !keys.is_empty() {
                let _: redis::RedisResult<()> = conn.del(keys).await;
            }
        }
    }
}