use prtl_messages::ProxyDescriptor;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ProxyRegistry {
    proxies: HashMap<String, ProxyDescriptor>,
}

impl ProxyRegistry {
    pub fn register(&mut self, descriptor: ProxyDescriptor) {
        tracing::info!("Registering proxy: {}", descriptor.service_name);
        self.proxies.insert(descriptor.service_name.clone(), descriptor);
    }

    pub fn find_proxy_for_domain(&self, domain: &str) -> Option<&ProxyDescriptor> {
        self.proxies.values().find(|desc| {
            desc.base_domains
                .iter()
                .any(|base_domain| domain == base_domain || domain.ends_with(&format!(".{}", base_domain)))
        })
    }
}
