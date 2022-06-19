use std::str::FromStr;

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
