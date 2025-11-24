use crate::handlers::handle_request;
use crate::registry::ProxyRegistry;
use crate::state::AppState;
use axum::Router;
use axum::routing::any;
use prtl_messages::BusMessage;
use std::sync::Arc;
use tracing::{error, info, warn};

mod cache_refresh;
mod error;
mod handlers;
mod hash;
mod registry;
mod state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,mirror_api=debug".into()),
        )
        .init();

    let nats_addr = std::env::var("NATS_ADDR").unwrap_or_else(|_| "nats://localhost:4222".into());
    let redis_addr = std::env::var("REDIS_ADDR").unwrap_or_else(|_| "redis://localhost:6379".into());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:80".into());

    info!("Connecting to NATS at {}", nats_addr);
    let nats = async_nats::connect(&nats_addr).await?;

    info!("Connecting to Redis/DragonflyDB at {}", redis_addr);
    let redis_client = redis::Client::open(redis_addr)?;
    let redis_conn = redis::aio::ConnectionManager::new(redis_client).await?;

    info!("Broadcasting discovery request");
    let discovery_subject = BusMessage::subject_for_discovery();
    let discovery_payload = rmp_serde::to_vec_named(&BusMessage::Discovery)?;
    nats.publish(discovery_subject, discovery_payload.into()).await?;

    let proxy_registry = Arc::new(tokio::sync::RwLock::new(ProxyRegistry::default()));

    let state = AppState {
        nats: nats.clone(),
        redis: redis_conn,
        proxy_registry: proxy_registry.clone(),
    };

    tokio::spawn(listen_for_proxy_registrations(nats.clone(), proxy_registry.clone()));

    let cache_refresh_config = cache_refresh::CacheRefreshConfig::default();
    let cache_refresh_service = cache_refresh::CacheRefreshService::new(state.redis.clone(), cache_refresh_config);
    tokio::spawn(cache_refresh_service.run());

    let app = Router::new().route("/{*path}", any(handle_request)).with_state(state);

    info!("Starting server on {}", bind_addr);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn listen_for_proxy_registrations(nats: async_nats::Client, registry: Arc<tokio::sync::RwLock<ProxyRegistry>>) {
    let mut sub = match nats.subscribe("mirror.proxy.*.register").await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to subscribe to proxy registrations: {}", e);
            return;
        }
    };

    info!("Listening for proxy registrations");

    while let Some(msg) = futures_util::stream::StreamExt::next(&mut sub).await {
        match rmp_serde::from_slice::<BusMessage>(&msg.payload) {
            Ok(BusMessage::RegisterParser(req)) => {
                let mut reg = registry.write().await;
                reg.register(req.descriptor);
            }
            Err(e) => {
                warn!("Failed to deserialize registration message: {}", e);
            }
            _ => {}
        }
    }
}
