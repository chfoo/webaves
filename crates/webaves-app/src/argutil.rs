use std::{net::SocketAddr, str::FromStr};

#[derive(Clone, Debug)]
pub struct DoHAddress(pub SocketAddr, pub String);

impl FromStr for DoHAddress {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split_once('/') {
            Some((address, hostname)) => {
                let address = address
                    .parse::<SocketAddr>()
                    .map_err(|error| error.to_string())?;
                Ok(DoHAddress(address, hostname.to_string()))
            }
            None => Err("bad DoH address format".to_string()),
        }
    }
}
