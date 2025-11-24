use crate::registry::ProxyRegistry;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub nats: async_nats::Client,
    pub redis: redis::aio::ConnectionManager,
    pub proxy_registry: Arc<tokio::sync::RwLock<ProxyRegistry>>,
}
