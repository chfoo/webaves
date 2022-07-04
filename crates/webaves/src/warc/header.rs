use std::{fmt::Display, str::FromStr};

use crate::header::HeaderMap;

use super::WARCError;

/// Helper trait for [HeaderMap].
pub trait HeaderMapExt {
    /// Returns a string or return an error.
    fn get_required(&self, name: &str) -> Result<&str, WARCError>;

    /// Returns a parsed value if available or return an error.
    fn get_parsed<T>(&self, name: &str) -> Result<Option<T>, WARCError>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static;

    /// Returns a parsed value or return an error.
    fn get_parsed_required<T>(&self, name: &str) -> Result<T, WARCError>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static;
}

impl HeaderMapExt for HeaderMap {
    fn get_required(&self, name: &str) -> Result<&str, WARCError> {
        match self.get(name) {
            Some(field) => Ok(&field.text),
            None => Err(make_field_error(self, name, None)),
        }
    }

    fn get_parsed<T>(&self, name: &str) -> Result<Option<T>, WARCError>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        match self.get(name) {
            Some(field) => field
                .text
                .parse::<T>()
                .map(|item| Some(item))
                .map_err(|error| make_field_error(self, name, Some(Box::new(error)))),
            None => Ok(None),
        }
    }

    fn get_parsed_required<T>(&self, name: &str) -> Result<T, WARCError>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        match self.get(name) {
            Some(field) => field
                .text
                .parse::<T>()
                .map_err(|error| make_field_error(self, name, Some(Box::new(error)))),
            None => Err(make_field_error(self, name, None)),
        }
    }
}

fn make_field_error(
    header: &HeaderMap,
    name: &str,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
) -> WARCError {
    WARCError::InvalidFieldValue {
        name: name.to_string(),
        record_id: header
            .get("WARC-Record-ID")
            .map(|field| field.text.as_str())
            .unwrap_or_default()
            .to_string(),
        source,
    }
}

/// Checksum or hashed value of some data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelledDigest {
    /// Algorithm name.
    pub algorithm: String,
    /// Value of the digest.
    pub value: Vec<u8>,
}

impl LabelledDigest {
    /// Create a `LabelledDigest` with the given values.
    pub fn new<A: Into<String>, V: Into<Vec<u8>>>(algorithm: A, value: V) -> Self {
        Self {
            algorithm: crate::crypto::normalize_hash_name(algorithm.into()),
            value: value.into(),
        }
    }
}

impl FromStr for LabelledDigest {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (left, right) = match s.split_once(':') {
            Some(result) => result,
            None => return Err(crate::error::Error::Misc("no separator")),
        };
        let left = left.trim();
        let right = right.trim();

        let name = crate::crypto::normalize_hash_name(left);

        let hex = data_encoding::HEXLOWER_PERMISSIVE.decode(right.as_bytes());
        let b32 = data_encoding::BASE32.decode(right.as_bytes());
        let value;

        match (hex, b32) {
            (Ok(hex), Ok(b32)) => {
                let is_uppercase = right
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());

                if is_uppercase {
                    value = b32;
                } else {
                    value = hex;
                }
            }
            (Ok(hex), Err(_)) => {
                value = hex;
            }
            (Err(_), Ok(b32)) => {
                value = b32;
            }
            (Err(_), Err(error)) => return Err(crate::error::Error::Other(Box::new(error))),
        }

        Ok(Self {
            algorithm: name,
            value,
        })
    }
}

impl Display for LabelledDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let encoded: String = {
            let b32 = data_encoding::BASE32.encode(&self.value);

            if b32.ends_with('=') {
                data_encoding::HEXLOWER.encode(&self.value)
            } else {
                b32
            }
        };

        f.write_fmt(format_args!("{}:{}", self.algorithm, encoded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_labelled_digest() {
        assert_eq!(
            LabelledDigest::from_str("md5:d41d8cd98f00b204e9800998ecf8427e").unwrap(),
            LabelledDigest::new(
                "md5",
                b"\xd4\x1d\x8c\xd9\x8f\x00\xb2\x04\xe9\x80\t\x98\xec\xf8B~".as_slice()
            )
        );

        assert_eq!(
            LabelledDigest::from_str("md5:2QOYZWMPACZAJ2MABGMOZ6CCPY======").unwrap(),
            LabelledDigest::new(
                "md5",
                b"\xd4\x1d\x8c\xd9\x8f\x00\xb2\x04\xe9\x80\t\x98\xec\xf8B~".as_slice()
            )
        );

        assert_eq!(
            LabelledDigest::from_str("SHA-1:da39a3ee5e6b4b0d3255bfef95601890afd80709").unwrap(),
            LabelledDigest::new(
                "sha1",
                b"\xda9\xa3\xee^kK\r2U\xbf\xef\x95`\x18\x90\xaf\xd8\x07\t".as_slice()
            )
        );
        assert_eq!(
            LabelledDigest::from_str("SHA-1:3I42H3S6NNFQ2MSVX7XZKYAYSCX5QBYJ").unwrap(),
            LabelledDigest::new(
                "sha1",
                b"\xda9\xa3\xee^kK\r2U\xbf\xef\x95`\x18\x90\xaf\xd8\x07\t".as_slice()
            )
        );
    }

    #[test]
    fn test_labelled_digest_invalid() {
        assert!(LabelledDigest::from_str("").is_err());
        assert!(LabelledDigest::from_str("a:f").is_err());
        assert!(LabelledDigest::from_str("a:X").is_err());
    }
}
