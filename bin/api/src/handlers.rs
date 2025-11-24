use crate::error::ApiError;
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::{OriginalUri, Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response as AxumResponse};
use http::{Request, Response};
use prtl_messages::BusMessage;
use redis::AsyncCommands;
use tracing::{error, info};
use url::Url;

pub async fn handle_request(
    State(state): State<AppState>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<AxumResponse, ApiError> {
    let raw_query = uri.query();
    let url = parse_url(&path, raw_query)?;

    let domain = url.domain().ok_or(ApiError::InvalidUrl("No domain".into()))?;

    let registry = state.proxy_registry.read().await;
    let proxy_desc = registry.find_proxy_for_domain(domain).ok_or_else(|| {
        error!("No proxy available for domain: {}", domain);
        ApiError::NoParserAvailable
    })?;

    let service_name = proxy_desc.service_name.clone();
    let cache_ttl_secs = proxy_desc.cache_ttl.map(|d| d.as_secs()).unwrap_or(3600); // todo: disable caching by default
    let hash_settings = proxy_desc.hash_settings;
    drop(registry);

    let mut req_builder_for_cache = Request::builder().method(method.as_str()).uri(url.as_str());

    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        if !name_str.eq_ignore_ascii_case("host") && !name_str.eq_ignore_ascii_case("connection") {
            req_builder_for_cache = req_builder_for_cache.header(name.as_str(), value.as_bytes());
        }
    }

    let request_for_cache = req_builder_for_cache.body(body.to_vec()).map_err(|e| {
        error!("Failed to build request for cache: {}", e);
        ApiError::InternalError(e.to_string())
    })?;

    let cache_hash = crate::hash::compute_cache_key(&request_for_cache, &hash_settings);
    let cache_key = format!("proxy:{}:{}", service_name, cache_hash);

    let mut redis = state.redis.clone();
    if let Ok(Some(cached_data)) = redis.get::<_, Option<Vec<u8>>>(&cache_key).await
        && let Ok((status_code, headers_vec, body)) =
            rmp_serde::from_slice::<(u16, Vec<(String, Vec<u8>)>, Vec<u8>)>(&cached_data)
    {
        info!("Cache hit for {} (key: {})", url, cache_key);

        let mut axum_headers = HeaderMap::new();
        for (name, value) in headers_vec {
            if let (Ok(header_name), Ok(header_value)) = (
                axum::http::HeaderName::try_from(name),
                axum::http::HeaderValue::from_bytes(&value),
            ) {
                axum_headers.insert(header_name, header_value);
            }
        }

        return Ok((
            StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK),
            axum_headers,
            body,
        )
            .into_response());
    }

    let mut req_builder = Request::builder().method(method.as_str()).uri(url.as_str());

    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        if !name_str.eq_ignore_ascii_case("host") && !name_str.eq_ignore_ascii_case("connection") {
            req_builder = req_builder.header(name.as_str(), value.as_bytes());
        }
    }

    let http_request = req_builder.body(body.to_vec()).map_err(|e| {
        error!("Failed to build request: {}", e);
        ApiError::InternalError(e.to_string())
    })?;

    let rpc_subject = BusMessage::subject_for_rpc(&service_name);
    let payload = rmp_serde::to_vec_named(&BusMessage::ProxyRequest(http_request)).map_err(|e| {
        error!("Serialization error: {}", e);
        ApiError::InternalError(e.to_string())
    })?;

    let response = state
        .nats
        .request(rpc_subject.clone(), payload.into())
        .await
        .map_err(|e| {
            error!("NATS request to {} failed: {}", rpc_subject, e);
            ApiError::InternalError(e.to_string())
        })?;

    let proxy_response: BusMessage = rmp_serde::from_slice(&response.payload).map_err(|e| {
        error!("Failed to deserialize proxy response: {}", e);
        ApiError::InternalError(e.to_string())
    })?;

    let http_response = match proxy_response {
        BusMessage::ProxyResponse(resp) => {
            info!("Proxy response OK, status={}", resp.status());
            resp
        }
        _ => {
            error!("Unexpected response type from proxy");
            return Err(ApiError::InternalError("Unexpected response".into()));
        }
    };

    if http_response.status().is_success() {
        let status_code = http_response.status().as_u16();
        let headers_vec: Vec<(String, Vec<u8>)> = http_response
            .headers()
            .iter()
            .map(|(name, value)| (name.as_str().to_string(), value.as_bytes().to_vec()))
            .collect();
        let body = http_response.body().clone();

        if let Ok(cached_data) = rmp_serde::to_vec(&(status_code, headers_vec, body)) {
            let _: Result<(), _> = redis.set_ex(&cache_key, cached_data, cache_ttl_secs).await;
        }
    }

    Ok(convert_response_to_axum(http_response))
}

fn convert_response_to_axum(response: Response<Vec<u8>>) -> AxumResponse {
    let (parts, body) = response.into_parts();
    let mut axum_headers = HeaderMap::new();

    for (name, value) in parts.headers.iter() {
        if let Ok(axum_value) = axum::http::HeaderValue::from_bytes(value.as_bytes()) {
            axum_headers.insert(name.clone(), axum_value);
        }
    }

    (parts.status, axum_headers, body).into_response()
}

fn parse_url(path: &str, raw_query: Option<&str>) -> Result<Url, ApiError> {
    let path = path.trim_start_matches('/');

    let parts: Vec<&str> = path.splitn(2, '/').collect();

    let domain = parts.first().ok_or(ApiError::InvalidPath)?;
    let resource_path = parts.get(1).map(|s| format!("/{}", s)).unwrap_or_default();

    let mut url_str = format!("https://{}{}", domain, resource_path);

    if let Some(query) = raw_query
        && !query.is_empty()
    {
        url_str.push('?');
        url_str.push_str(query);
    }

    Url::parse(&url_str).map_err(|e| ApiError::InvalidUrl(e.to_string()))
}
