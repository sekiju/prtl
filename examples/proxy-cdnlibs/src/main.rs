use flate2::read::GzDecoder;
use http::{Request, Response, StatusCode};
use prtl_proxy::messages::{HashComponents, ProxyDescriptor};
use prtl_proxy::utils::json::{FieldFilter, filter_top_level_fields};
use prtl_proxy::{BoxError, PrtlService};
use std::io::Read;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let service = Arc::new(S::default());

    info!("Starting proxy-cdnlibs service");
    prtl_proxy::serve(service).await?;

    Ok(())
}

#[derive(Debug, Clone)]
struct RateLimiter {
    limit: Option<u64>,
    remaining: Option<u64>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            limit: None,
            remaining: None,
        }
    }

    fn update(&mut self, limit: u64, remaining: u64) {
        self.limit = Some(limit);
        self.remaining = Some(remaining);
    }

    fn should_throttle(&self) -> bool {
        if let Some(remaining) = self.remaining {
            remaining == 0
        } else {
            false
        }
    }
}

pub struct S {
    client: reqwest::Client,
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

impl Default for S {
    fn default() -> Self {
        Self::new()
    }
}

impl S {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder().use_rustls_tls().build().unwrap(),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new())),
        }
    }

    async fn validate_request(&self, req: &Request<Vec<u8>>) -> bool {
        if let Some(path) = req.uri().path_and_query() {
            return path.as_str().starts_with("/api/anime/");
        }

        false
    }

    async fn execute_request(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, BoxError> {
        {
            let limiter = self.rate_limiter.read().await;
            if limiter.should_throttle() {
                warn!("Rate limit exceeded, returning 429");
                return Ok(Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("Retry-After", "60")
                    .body(b"Rate limit exceeded".to_vec())?);
            }
        }

        let method = request.method().clone();
        let uri = request.uri().to_string();

        let mut req_builder = self.client.request(method, &uri);

        for (name, value) in request.headers() {
            if let Ok(value_str) = value.to_str() {
                req_builder = req_builder.header(name.as_str(), value_str);
            }
        }

        req_builder = req_builder.body(request.body().clone());

        let response = req_builder.send().await?;

        let rate_limit = response
            .headers()
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        let rate_remaining = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        if let (Some(limit), Some(remaining)) = (rate_limit, rate_remaining) {
            let mut limiter = self.rate_limiter.write().await;
            limiter.update(limit, remaining);
            info!("Rate limit updated: {}/{}", remaining, limit);
        }

        let status = response.status();
        let mut resp_builder = Response::builder().status(status);

        let is_json = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("application/json"))
            .unwrap_or(false);

        let is_gzipped = response
            .headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("gzip"))
            .unwrap_or(false);

        for (name, value) in response.headers() {
            resp_builder = resp_builder.header(name.as_str(), value.as_bytes());
        }

        let body = response.bytes().await?.to_vec();

        let filtered_body = if is_json && !body.is_empty() {
            let decompressed = if is_gzipped {
                let mut decoder = GzDecoder::new(&body[..]);
                let mut decompressed = Vec::new();
                match decoder.read_to_end(&mut decompressed) {
                    Ok(_) => {
                        info!(
                            "Decompressed gzipped response ({} -> {} bytes)",
                            body.len(),
                            decompressed.len()
                        );
                        decompressed
                    }
                    Err(e) => {
                        warn!("Failed to decompress gzipped response: {}", e);
                        return Ok(resp_builder.body(body)?);
                    }
                }
            } else {
                body.clone()
            };

            let trimmed = decompressed.iter().take_while(|&&b| b.is_ascii_whitespace()).count();
            if trimmed == decompressed.len() {
                body
            } else {
                match filter_top_level_fields(&decompressed, &FieldFilter::Deny(vec!["meta".to_string()])) {
                    Ok(filtered) => {
                        info!("Successfully filtered 'meta' field from response");

                        if is_gzipped {
                            use flate2::Compression;
                            use flate2::write::GzEncoder;
                            use std::io::Write;

                            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                            if let Err(e) = encoder.write_all(&filtered) {
                                warn!("Failed to re-compress response: {}", e);
                                return Ok(resp_builder.body(body)?);
                            }
                            match encoder.finish() {
                                Ok(compressed) => {
                                    info!(
                                        "Re-compressed filtered response ({} -> {} bytes)",
                                        filtered.len(),
                                        compressed.len()
                                    );
                                    compressed
                                }
                                Err(e) => {
                                    warn!("Failed to finish compression: {}", e);
                                    return Ok(resp_builder.body(body)?);
                                }
                            }
                        } else {
                            filtered
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to filter JSON response (body_len: {}, first_bytes: {:?}), returning original: {}",
                            decompressed.len(),
                            &decompressed.get(..std::cmp::min(20, decompressed.len())),
                            e
                        );
                        body
                    }
                }
            }
        } else {
            body
        };

        Ok(resp_builder.body(filtered_body)?)
    }
}

#[async_trait::async_trait]
impl PrtlService for S {
    fn descriptor(&self) -> ProxyDescriptor {
        ProxyDescriptor {
            service_name: "cdnlibs".into(),
            base_domains: vec!["api.cdnlibs.org".into()],
            hash_settings: HashComponents::URL | HashComponents::QUERY,
            cache_ttl: Some(std::time::Duration::from_secs(3600)), // 1 hour
        }
    }

    async fn handle_request(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, BoxError> {
        if !self.validate_request(&request).await {
            let resp = http::Response::builder().status(403).body(Vec::new()).unwrap();
            return Ok(resp);
        }

        self.execute_request(request).await
    }
}
