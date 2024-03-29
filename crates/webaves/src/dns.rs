//! DNS client facade.

use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

use rand::Rng;
use serde::{Deserialize, Serialize};
use trust_dns_resolver::{
    config::{LookupIpStrategy, NameServerConfig, Protocol, ResolverConfig, ResolverOpts},
    error::{ResolveError, ResolveErrorKind},
    lookup_ip::LookupIp,
    proto::{op::ResponseCode, rr::RecordType},
    Resolver as TrustResolver,
};

/// DNS resolver client with a simple interface.
///
/// The client is intended for archiving purposes. As such, it does not use
/// the system's resolver. The implementation uses an external crate
/// configured to sensible values.
///
/// Results are automatically cached.
pub struct Resolver {
    inner: TrustResolver,
}

impl Resolver {
    fn new(inner: TrustResolver) -> Self {
        Self { inner }
    }

    /// Return a builder for configuring a new instance.
    pub fn builder() -> ResolverBuilder {
        ResolverBuilder::new()
    }

    /// Resolve the given hostname to IP addresses.
    #[tracing::instrument(skip(self), level = "debug")]
    pub fn lookup_address<S>(&self, hostname: S) -> Result<AddressResponse, ResolverError>
    where
        S: AsRef<str> + std::fmt::Debug,
    {
        let result = self.inner.lookup_ip(hostname.as_ref());

        match result {
            Ok(items) => self.process_address_ok(items),
            Err(error) => self.process_address_err(error),
        }
    }

    fn process_address_ok(&self, items: LookupIp) -> Result<AddressResponse, ResolverError> {
        let mut address_response = AddressResponse::default();

        address_response.addresses.extend(items.iter());

        for record in items.as_lookup().record_iter() {
            address_response.text_records.push(format!("{}", record));
        }

        tracing::debug!(count = address_response.addresses.len(), "ok");

        Ok(address_response)
    }

    fn process_address_err(&self, error: ResolveError) -> Result<AddressResponse, ResolverError> {
        if let ResolveErrorKind::NoRecordsFound {
            query: _,
            soa: _,
            negative_ttl: _,
            response_code,
            trusted: _,
        } = error.kind()
        {
            tracing::debug!(response_code = response_code.to_str(), "err");
        }

        Err(error.into())
    }

    /// Resolve the given hostname to DNS resource records.
    #[tracing::instrument(skip(self), level = "debug")]
    pub fn lookup_record<R, H>(
        &self,
        record_type: R,
        hostname: H,
    ) -> Result<Vec<String>, ResolverError>
    where
        R: AsRef<str> + std::fmt::Debug,
        H: AsRef<str> + std::fmt::Debug,
    {
        let record_type = Self::parse_record_type(record_type.as_ref())?;

        let response = self.inner.lookup(hostname.as_ref(), record_type)?;
        let mut text_records = Vec::new();

        for record in response.record_iter() {
            text_records.push(record.to_string())
        }

        Ok(text_records)
    }

    fn parse_record_type(record_type: &str) -> Result<RecordType, ResolverError> {
        if let Ok(value) = record_type.parse::<u16>() {
            return Ok(RecordType::from(value));
        }

        match RecordType::from_str(record_type) {
            Ok(value) => Ok(value),
            Err(error) => Err(ResolverError::InvalidArg(Box::new(error))),
        }
    }

    /// Removes any stored entires in the cache.
    pub fn clear_cache(&mut self) {
        self.inner.clear_cache().unwrap();
    }
}

/// Configures and creates a [`Resolver`].
pub struct ResolverBuilder {
    bind_address: Option<SocketAddr>,
    doh_servers: Vec<(SocketAddr, String)>,
    dnssec: bool,
}

impl Default for ResolverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolverBuilder {
    /// Creates a `ResolverBuilder with the default configuration.
    pub fn new() -> Self {
        Self {
            bind_address: None,
            doh_servers: Vec::new(),
            dnssec: false,
        }
    }

    /// Set the outgoing network interface address.
    ///
    /// Default is None.
    pub fn with_bind_address(mut self, address: SocketAddr) -> Self {
        self.bind_address = Some(address);
        self
    }

    /// Add a DNS-over-HTTPS server.
    ///
    /// Default is no servers.
    pub fn with_doh_server(mut self, address: SocketAddr, hostname: &str) -> Self {
        self.doh_servers.push((address, hostname.to_string()));
        self
    }

    /// Enable DNSSEC.
    ///
    /// Default is false.
    pub fn with_dnssec(mut self, value: bool) -> Self {
        self.dnssec = value;
        self
    }

    /// Create a configured instance.
    pub fn build(&self) -> Resolver {
        let mut opts = ResolverOpts::default();
        opts.timeout = Duration::from_secs(10);
        opts.attempts = 1;
        opts.edns0 = true;
        opts.ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
        opts.cache_size = 128;
        opts.use_hosts_file = false;
        opts.preserve_intermediates = true;

        let mut config = ResolverConfig::new();

        for server in &self.doh_servers {
            let server_config = NameServerConfig {
                socket_addr: server.0,
                protocol: Protocol::Https,
                tls_dns_name: Some(server.1.to_string()),
                trust_nx_responses: false,
                tls_config: None,
                bind_addr: self.bind_address,
            };

            config.add_name_server(server_config);
        }

        Resolver::new(TrustResolver::new(config, opts).unwrap())
    }
}

/// IP address lookup response.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AddressResponse {
    addresses: Vec<IpAddr>,
    text_records: Vec<String>,
}

impl AddressResponse {
    /// Resolved IP addresses.
    pub fn addresses(&self) -> &[IpAddr] {
        &self.addresses
    }

    /// Resource records in textual format
    pub fn text_records(&self) -> &[String] {
        &self.text_records
    }
}

/// General DNS resolver errors.
#[derive(thiserror::Error, Debug)]
pub enum ResolverError {
    /// Non-existent domain.
    #[error("non-existent domain")]
    NoName(#[source] ResolveError),

    /// No records for given record type.
    #[error("no records for given record type")]
    NoRecord(#[source] ResolveError),

    /// Other negative response.
    #[error(transparent)]
    Negative(ResolveError),

    /// Protocol error.
    #[error(transparent)]
    Protocol(ResolveError),

    /// Invalid argument
    #[error("invalid argument")]
    InvalidArg(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Standard IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<ResolveError> for ResolverError {
    fn from(error: ResolveError) -> Self {
        match error.kind() {
            ResolveErrorKind::NoRecordsFound {
                query: _,
                soa: _,
                negative_ttl: _,
                response_code: ResponseCode::NXDomain,
                trusted: _,
            } => Self::NoName(error),
            ResolveErrorKind::NoRecordsFound {
                query: _,
                soa: _,
                negative_ttl: _,
                response_code: ResponseCode::NoError,
                trusted: _,
            } => Self::NoRecord(error),
            ResolveErrorKind::NoRecordsFound {
                query: _,
                soa: _,
                negative_ttl: _,
                response_code: _,
                trusted: _,
            } => Self::Negative(error),
            _ => Self::Protocol(error),
        }
    }
}

/// Generate a domain name that is unlikely to exist.
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

    #[test_log::test]
    #[ignore = "external resources"]
    fn test_resolver() {
        let resolver = ResolverBuilder::new()
            .with_doh_server("1.1.1.1:443".parse().unwrap(), "cloudflare-dns.com")
            .with_doh_server("8.8.8.8:443".parse().unwrap(), "dns.google")
            .build();

        let result = resolver.lookup_address("www.icanhascheezburger.com");
        assert!(matches!(result, Ok(_)));

        let lookup = result.unwrap();
        assert!(!lookup.addresses.is_empty());
        assert!(!lookup.text_records.is_empty());

        let result = resolver.lookup_address(&random_domain());
        assert!(matches!(result, Err(ResolverError::NoName(_))));
    }
}
