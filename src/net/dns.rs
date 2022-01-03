use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use rand::Rng;
use thiserror::Error;
use tracing::debug;
use trust_dns_resolver::{
    config::{LookupIpStrategy, NameServerConfig, ResolverConfig, ResolverOpts},
    lookup_ip::LookupIp,
    proto::rr::Record,
};

pub struct Resolver {
    inner: trust_dns_resolver::TokioAsyncResolver,
    cache: HashMap<String, CacheEntry>,
}

impl Resolver {
    fn new(inner: trust_dns_resolver::TokioAsyncResolver) -> Resolver {
        Self {
            inner,
            cache: HashMap::new(),
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn lookup_address(&mut self, hostname: &str) -> Result<AddressLookup, ResolverError> {
        self.clean_cache();

        if let Some(entry) = self.cache.get(hostname) {
            debug!("cached entry");

            if entry.exists {
                return Ok(AddressLookup {
                    addresses: entry.addresses.clone(),
                    records: None,
                });
            } else {
                return Err(ResolverError::NotFound);
            }
        };

        debug!("lookup ip");

        match self.inner.lookup_ip(hostname).await {
            Ok(query_result) => Ok(self.process_ok_query(hostname, query_result)),
            Err(error) => Err(self.process_err_query(hostname, error)),
        }
    }

    fn process_ok_query(&mut self, hostname: &str, query_result: LookupIp) -> AddressLookup {
        let addresses = query_result.iter().collect::<Vec<IpAddr>>();
        let expiry = query_result.valid_until();
        let records = query_result
            .as_lookup()
            .record_iter()
            .cloned()
            .collect::<Vec<Record>>();

        self.insert_cache(hostname, addresses.clone(), expiry);

        debug!(?addresses);

        AddressLookup {
            addresses,
            records: Some(records),
        }
    }

    fn process_err_query(
        &mut self,
        hostname: &str,
        error: trust_dns_resolver::error::ResolveError,
    ) -> ResolverError {
        debug!(?error);

        match error.kind() {
            trust_dns_resolver::error::ResolveErrorKind::NoRecordsFound {
                query: _,
                soa: _,
                negative_ttl,
                response_code: _,
                trusted: _,
            } => {
                self.insert_negative_cache(hostname, negative_ttl.unwrap_or(60));
                ResolverError::NotFound
            }
            _ => ResolverError::Other(error),
        }
    }

    fn clean_cache(&mut self) {
        self.cache.retain(|_k, v| v.expiry >= Instant::now());
    }

    fn insert_cache(&mut self, hostname: &str, addresses: Vec<IpAddr>, expiry: Instant) {
        let expiry = expiry.min(Instant::now() + Duration::from_secs(3600));

        self.cache.insert(
            hostname.to_string(),
            CacheEntry {
                addresses,
                expiry,
                exists: true,
            },
        );
    }

    fn insert_negative_cache(&mut self, hostname: &str, ttl: u32) {
        let expiry = Instant::now() + Duration::from_secs(ttl.min(3600).into());

        self.cache.insert(
            hostname.to_string(),
            CacheEntry {
                addresses: Vec::new(),
                expiry,
                exists: false,
            },
        );
    }
}

struct CacheEntry {
    addresses: Vec<IpAddr>,
    expiry: Instant,
    exists: bool,
}

pub struct AddressLookup {
    pub addresses: Vec<IpAddr>,
    pub records: Option<Vec<Record>>,
}

pub struct Builder {
    config: ResolverConfig,
}

impl Builder {
    pub fn new() -> Builder {
        Self {
            config: ResolverConfig::new(),
        }
    }

    pub fn with_doh_server(mut self, address: IpAddr, port: u16, hostname: &str) -> Self {
        self.config.add_name_server(NameServerConfig {
            socket_addr: SocketAddr::new(address, port),
            protocol: trust_dns_resolver::config::Protocol::Https,
            tls_dns_name: Some(hostname.to_string()),
            trust_nx_responses: false,
            tls_config: None,
        });
        self
    }

    pub fn build(&self) -> Resolver {
        let options = ResolverOpts {
            timeout: Duration::from_secs(10),
            edns0: true,
            ip_strategy: LookupIpStrategy::Ipv4AndIpv6,
            use_hosts_file: false,
            preserve_intermediates: true,
            ..Default::default()
        };

        Resolver::new(
            trust_dns_resolver::TokioAsyncResolver::tokio(self.config.clone(), options).unwrap(),
        )
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Error, Debug)]
pub enum ResolverError {
    #[error("Not found")]
    NotFound,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] trust_dns_resolver::error::ResolveError),
}

pub fn random_domain() -> String {
    let length = rand::thread_rng().gen_range(20usize..=50usize);
    let label = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect::<String>();

    format!("{}.net", label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_domain() {
        let result = random_domain();

        assert!(result.len() > 20);
        assert!(result.len() < 60);
        assert!(result.contains('.'));
    }
}
