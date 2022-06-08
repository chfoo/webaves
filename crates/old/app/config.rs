use std::net::IpAddr;

use serde::Deserialize;

use crate::dns;

#[derive(Deserialize)]
pub struct RunConfig {
    pub dns: DnsConfig,
}

#[derive(Deserialize)]
pub struct DnsConfig {
    pub doh_servers: Vec<DohServer>,
}

impl DnsConfig {
    pub fn make_resolver(&self) -> dns::Resolver {
        let mut builder = dns::Builder::new();

        for server in &self.doh_servers {
            builder = builder.with_doh_server(server.address, server.port, &server.hostname);
        }

        builder.build()
    }
}

#[derive(Deserialize)]
pub struct DohServer {
    pub address: IpAddr,
    pub port: u16,
    pub hostname: String,
}
