use simd_json::OwnedValue;
use simd_json::base::ValueAsMutObject;

#[derive(Debug, Clone)]
pub enum FieldFilter {
    Allow(Vec<String>),
    Deny(Vec<String>),
}

pub fn filter_top_level_fields(body: &[u8], filter: &FieldFilter) -> Result<Vec<u8>, simd_json::Error> {
    let mut body_copy = body.to_vec();
    let mut value: OwnedValue = simd_json::to_owned_value(&mut body_copy)?;

    let obj = match value.as_object_mut() {
        Some(o) => o,
        None => return Ok(body.to_vec()),
    };

    match filter {
        FieldFilter::Allow(allow) => {
            let allow_set: std::collections::HashSet<&str> = allow.iter().map(|s| s.as_str()).collect();
            obj.retain(|k, _| allow_set.contains(k.as_str()));
        }
        FieldFilter::Deny(deny) => {
            for k in deny {
                obj.remove(k.as_str());
            }
        }
    }

    let mut out = Vec::with_capacity(body.len());
    simd_json::to_writer(&mut out, &value)?;
    Ok(out)
}
