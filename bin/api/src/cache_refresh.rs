use redis::AsyncCommands;
use std::time::Duration;
use tracing::{debug, error, info};

pub struct CacheRefreshConfig {
    pub refresh_interval_seconds: u64,
    pub refresh_threshold_ratio: f64,
    pub max_refresh_per_scan: usize,
}

impl Default for CacheRefreshConfig {
    fn default() -> Self {
        Self {
            refresh_interval_seconds: 60,
            refresh_threshold_ratio: 0.8,
            max_refresh_per_scan: 10,
        }
    }
}

pub struct CacheRefreshService {
    redis: redis::aio::ConnectionManager,
    config: CacheRefreshConfig,
}

impl CacheRefreshService {
    pub fn new(redis: redis::aio::ConnectionManager, config: CacheRefreshConfig) -> Self {
        Self { redis, config }
    }

    pub async fn run(mut self) {
        info!(
            "Starting cache refresh service with interval {}s",
            self.config.refresh_interval_seconds
        );

        let mut interval = tokio::time::interval(Duration::from_secs(self.config.refresh_interval_seconds));

        loop {
            interval.tick().await;

            if let Err(e) = self.refresh_old_cache_entries().await {
                error!("Cache refresh error: {}", e);
            }
        }
    }

    async fn refresh_old_cache_entries(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut redis = self.redis.clone();

        let keys: Vec<String> = redis.keys("proxy:*").await?;

        if keys.is_empty() {
            debug!("No cache entries found");
            return Ok(());
        }

        debug!("Scanning {} cache entries", keys.len());

        let mut refreshed_count = 0;

        for key in keys.iter().take(self.config.max_refresh_per_scan) {
            if refreshed_count >= self.config.max_refresh_per_scan {
                break;
            }

            let ttl: i64 = redis.ttl(key.as_str()).await.unwrap_or(-1);

            if ttl > 0 && ttl < (3600.0 * self.config.refresh_threshold_ratio) as i64 {
                debug!("Cache entry {} has low TTL: {}s", key, ttl);
                refreshed_count += 1;
            }
        }

        if refreshed_count > 0 {
            info!("Found {} cache entries to potentially refresh", refreshed_count);
        }

        Ok(())
    }
}
