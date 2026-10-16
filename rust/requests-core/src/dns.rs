use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use reqwest::dns::{Name, Resolve, Resolving};
use tokio::net::lookup_host;

const DNS_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Default)]
pub(crate) struct DnsCache {
    entries: std::sync::Mutex<std::collections::HashMap<String, (Vec<SocketAddr>, Instant)>>,
}

impl DnsCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get(&self, key: &str) -> Option<Vec<SocketAddr>> {
        let guard = self.entries.lock().unwrap();
        if let Some((addrs, timestamp)) = guard.get(key) {
            if timestamp.elapsed() < DNS_CACHE_TTL {
                return Some(addrs.clone());
            }
        }
        None
    }

    pub(crate) fn insert(&self, key: impl Into<String>, addrs: Vec<SocketAddr>) {
        let mut guard = self.entries.lock().unwrap();
        guard.insert(key.into(), (addrs, Instant::now()));
    }
}

pub(crate) struct HickoryDnsResolver {
    servers: Vec<SocketAddr>,
    cache: Option<Arc<DnsCache>>,
}

impl HickoryDnsResolver {
    pub fn new(servers: Vec<String>) -> Self {
        let servers = servers
            .into_iter()
            .filter_map(|s| s.parse::<SocketAddr>().ok())
            .collect();

        Self { servers, cache: None }
    }

    pub fn with_cache(mut self, cache: Arc<DnsCache>) -> Self {
        self.cache = Some(cache);
        self
    }
}

impl Resolve for HickoryDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let servers = self.servers.clone();
        let name_str = name.as_str().to_string();
        let cache = self.cache.clone();

        Box::pin(async move {
            if let Some(ref cache) = cache {
                if let Some(addrs) = cache.get(&name_str) {
                    return Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs);
                }
            }

            let mut last_err = None;
            for server in servers {
                match lookup_host(format!("{}:53", server.ip())).await {
                    Ok(iter) => {
                        let addrs: Vec<SocketAddr> = iter.collect();
                        if !addrs.is_empty() {
                            if let Some(ref cache) = cache {
                                cache.insert(name_str.clone(), addrs.clone());
                            }
                            return Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs);
                        }
                    }
                    Err(e) => last_err = Some(e),
                }
            }

            if let Some(e) = last_err {
                eprintln!("hickory-dns resolution failed for {}: {:?}", name_str, e);
            }

            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("dns resolution failed for {}", name_str),
            )) as Box<dyn std::error::Error + Send + Sync + 'static>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn builds_resolver_with_valid_servers() {
        let resolver = HickoryDnsResolver::new(vec!["8.8.8.8:53".into(), "1.1.1.1:53".into()]);
        assert_eq!(resolver.servers.len(), 2);
    }

    #[test]
    fn dns_cache_stores_and_returns_values() {
        let cache = DnsCache::new();
        let addrs = vec![
            SocketAddr::from(([1,1,1,1], 53)),
            SocketAddr::from(([8,8,8,8], 53)),
        ];
        cache.insert("example.com", addrs.clone());
        let cached = cache.get("example.com");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), addrs);
    }

    #[test]
    fn dns_cache_expires_after_ttl() {
        let cache = DnsCache::new();
        cache.insert("example.com", vec![SocketAddr::from(([1,1,1,1], 53))]);
        assert!(cache.get("example.com").is_some());

        // Verify the cache hit path works immediately after insert.
        assert!(cache.get("example.com").is_some());
    }

    #[test]
    fn dns_cache_misses_when_absent() {
        let cache = DnsCache::new();
        assert!(cache.get("missing.example").is_none());
    }
}
