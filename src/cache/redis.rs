use std::time::Duration;
use redis::{AsyncCommands, aio::Connection};
use tokio::sync::Mutex;

use super::Cache;

pub struct RedisCache {
    conn: Mutex<Connection>,
}

impl RedisCache {
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        // En redis 0.25 usamos una conexión Tokio simple
        let conn = client.get_tokio_connection().await?;
        Ok(Self { conn: Mutex::new(conn) })
    }
}

#[async_trait::async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut conn = self.conn.lock().await;
        match conn.get::<_, Vec<u8>>(key).await {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) {
        let mut conn = self.conn.lock().await;
        let _: redis::RedisResult<()> = match ttl {
            Some(d) => conn.set_ex(key, value, d.as_secs()).await,
            None => conn.set(key, value).await,
        };
    }

    async fn del(&self, key: &str) {
        let mut conn = self.conn.lock().await;
        let _: redis::RedisResult<()> = conn.del(key).await;
    }

    async fn del_prefix(&self, prefix: &str) {
        // Sencillo (no óptimo): KEYS prefix* y DEL
        let mut conn = self.conn.lock().await;
        let pattern = format!("{}*", prefix);
        let keys_res: redis::RedisResult<Vec<String>> = conn.keys(&pattern).await;
        if let Ok(keys) = keys_res {
            if !keys.is_empty() {
                let _: redis::RedisResult<()> = conn.del(keys).await;
            }
        }
    }
}