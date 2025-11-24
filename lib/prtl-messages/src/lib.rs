use bitflags::bitflags;
use http::{Request, Response};
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct HashComponents: u8 {
        const URL = 0b0001;
        const QUERY = 0b0010;
        const HEADERS = 0b0100;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyDescriptor {
    pub service_name: String,
    pub base_domains: Vec<String>,
    pub hash_settings: HashComponents,
    pub cache_ttl: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParserCapabilities {
    Rest,
    GraphQl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterProxyRequest {
    pub descriptor: ProxyDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterProxyReply {
    pub accepted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BusMessage {
    RegisterParser(RegisterProxyRequest),
    RegisterParserReply(RegisterProxyReply),
    ProxyRequest(#[serde(with = "http_serde_ext::request")] Request<Vec<u8>>),
    ProxyResponse(#[serde(with = "http_serde_ext::response")] Response<Vec<u8>>),
    Discovery,
}

impl BusMessage {
    pub fn subject_for_register(service: &str) -> String {
        format!("prtl.proxy.{service}.register")
    }

    pub fn subject_for_rpc(service: &str) -> String {
        format!("prtl.proxy.{service}.rpc")
    }

    pub fn subject_for_discovery() -> String {
        "prtl.discovery".to_string()
    }
}
