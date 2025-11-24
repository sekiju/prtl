use http::Request;
use prtl_messages::HashComponents;

pub fn compute_cache_key<B>(request: &Request<B>, settings: &HashComponents) -> String
where
    B: AsRef<[u8]>,
{
    let mut hasher = blake3::Hasher::new();

    if settings.contains(HashComponents::URL) {
        hasher.update(request.uri().path().as_bytes());
    }

    if settings.contains(HashComponents::QUERY)
        && let Some(query) = request.uri().query()
    {
        hasher.update(query.as_bytes());
    }

    if settings.contains(HashComponents::HEADERS) {
        let mut headers: Vec<_> = request
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_bytes()))
            .collect();
        headers.sort_by_key(|(k, _)| *k);

        for (key, value) in headers {
            hasher.update(key.as_bytes());
            hasher.update(b":");
            hasher.update(value);
            hasher.update(b"\n");
        }
    }

    hasher.finalize().to_hex().to_string()
}
