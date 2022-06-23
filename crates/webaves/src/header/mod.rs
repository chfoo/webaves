//! HTTP-style header name-value fields.
//!
//! Supports awareness of the following:
//!
//! - spaces before colon
//! - folded lines
//! - quoted-string
//! - encoded-word (RFC2047)
//!
//! Note that the data structures do not perform validation on their own and
//! are allowed to hold potentially malformed or invalid character sequences.
mod format;
mod parse;
mod pc;
mod util;

pub use format::*;
pub use parse::*;
pub use util::*;

use std::{collections::VecDeque, fmt::Display, ops::Index};

use serde::{Deserialize, Serialize};

use crate::string::StringLosslessExt;

/// Multimap of name-value fields.
///
/// This container is a multimap where multiple values may be associated with
/// the same name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "Vec<FieldPair>")]
#[serde(into = "Vec<FieldPair>")]
pub struct HeaderMap {
    pairs: Vec<FieldPair>,
}

impl HeaderMap {
    /// Creates an empty `HeaderMap`.
    pub fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    /// Returns the number of fields.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Returns whether the container has no fields.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator of all fields.
    pub fn iter(&self) -> HeaderMapIter<'_> {
        HeaderMapIter {
            map: self,
            index: 0,
        }
    }

    /// Returns whether a field with the given name exists in the container.
    pub fn contains_key<N: Into<String>>(&self, name: N) -> bool {
        self.get(name).is_some()
    }

    /// Returns the first field value for the given name.
    pub fn get<N: Into<String>>(&self, name: N) -> Option<&FieldValue> {
        let mut name = name.into();
        name.make_ascii_lowercase();

        for pair in &self.pairs {
            if pair.name.normalized == name {
                return Some(&pair.value);
            }
        }

        None
    }

    /// Returns all the field values for the given name.
    pub fn get_all<N: Into<String>>(&self, name: N) -> FieldValuesIter<'_> {
        let mut name = name.into();
        name.make_ascii_lowercase();

        FieldValuesIter {
            name,
            map: self,
            index: 0,
        }
    }

    /// Returns the the first value as a string for the given name.
    pub fn get_str<N: Into<String>>(&self, name: N) -> Option<&str> {
        match self.get(name) {
            Some(field) => Some(field.text.as_ref()),
            None => None,
        }
    }

    /// Add a field preserving any fields matching the given name.
    pub fn append<N, V>(&mut self, name: N, value: V)
    where
        N: Into<FieldName>,
        V: Into<FieldValue>,
    {
        self.pairs.push(FieldPair::new(name.into(), value.into()))
    }

    /// Remove any existing field with the given name and add the given field.
    pub fn insert<N, V>(&mut self, name: N, value: V)
    where
        N: Into<FieldName>,
        V: Into<FieldValue>,
    {
        let name = name.into();
        self.pairs
            .retain(|pair| pair.name.normalized != name.normalized);
        self.pairs.push(FieldPair::new(name, value.into()));
    }

    /// Moves the fields to the front of the list order.
    ///
    /// Some servers may sensitive to fields such as "Host" or "Date" being
    /// the first field, so this function provides a way to easily move
    /// them to the front.
    pub fn reorder_front(&mut self, name: &str) {
        let name = name.to_ascii_lowercase();
        let mut input_pairs = VecDeque::from(std::mem::take(&mut self.pairs));
        let mut high_priority = VecDeque::new();
        let mut low_priority = VecDeque::new();

        self.pairs.reserve(input_pairs.len());

        while let Some(pair) = input_pairs.pop_front() {
            if name == pair.name.normalized {
                high_priority.push_back(pair);
            } else {
                low_priority.push_back(pair);
            }
        }

        while let Some(pair) = high_priority.pop_front() {
            self.pairs.push(pair);
        }
        while let Some(pair) = low_priority.pop_front() {
            self.pairs.push(pair);
        }
    }
}

impl Default for HeaderMap {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Into<String>> Index<N> for HeaderMap {
    type Output = FieldValue;

    fn index(&self, index: N) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl From<Vec<FieldPair>> for HeaderMap {
    fn from(pairs: Vec<FieldPair>) -> Self {
        Self { pairs }
    }
}

impl From<HeaderMap> for Vec<FieldPair> {
    fn from(header: HeaderMap) -> Self {
        header.pairs
    }
}

impl Display for HeaderMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for pair in self.pairs.iter() {
            pair.fmt(f)?;
        }

        Ok(())
    }
}

/// Iterator for all fields.
pub struct HeaderMapIter<'a> {
    map: &'a HeaderMap,
    index: usize,
}

impl<'a> Iterator for HeaderMapIter<'a> {
    type Item = &'a FieldPair;

    fn next(&mut self) -> Option<Self::Item> {
        match self.map.pairs.get(self.index) {
            Some(item) => {
                self.index += 1;
                Some(item)
            }
            None => None,
        }
    }
}

/// Iterator of values for a name.
pub struct FieldValuesIter<'a> {
    name: String,
    map: &'a HeaderMap,
    index: usize,
}

impl<'a> Iterator for FieldValuesIter<'a> {
    type Item = &'a FieldValue;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.map.pairs.get(self.index) {
                Some(item) => {
                    self.index += 1;

                    if self.name == item.name.normalized {
                        return Some(&item.value);
                    }
                }
                None => return None,
            }
        }
    }
}

/// Represents a single name-value field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPair {
    /// The name or key.
    pub name: FieldName,
    /// The value.
    pub value: FieldValue,
}

impl FieldPair {
    /// Creates a `FieldPair` using the given name and value.
    pub fn new(name: FieldName, value: FieldValue) -> Self {
        Self { name, value }
    }
}

impl From<(FieldName, FieldValue)> for FieldPair {
    fn from(pair: (FieldName, FieldValue)) -> Self {
        FieldPair::new(pair.0, pair.1)
    }
}

impl Display for FieldPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name.to_text_lossy())?;
        f.write_str(": ")?;
        f.write_str(&self.value.to_text_lossy())?;
        f.write_str("\r\n")?;
        Ok(())
    }
}

/// Represents the name or key portion of a field.
///
/// The contents may be contain malformed or invalid sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "FieldNameDe")]
pub struct FieldName {
    #[serde(skip)]
    normalized: String,

    /// Name decoded.
    pub text: String,

    /// Name in the original encoded format.
    pub raw: Option<Vec<u8>>,
}

impl FieldName {
    /// Creates a `FieldName` with the given text and optional raw value.
    pub fn new(text: String, raw: Option<Vec<u8>>) -> Self {
        Self {
            normalized: text.to_ascii_lowercase(),
            text,
            raw,
        }
    }

    /// Returns a string with potential invalid characters replaced.
    ///
    /// This is intended for debugging purposes.
    pub fn to_text_lossy(&self) -> String {
        self.text.replace(|c| !(c as u8).is_token(), "\u{FFFD}")
    }
}

impl From<&str> for FieldName {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<String> for FieldName {
    fn from(value: String) -> Self {
        Self {
            normalized: value.to_ascii_lowercase(),
            text: value,
            raw: None,
        }
    }
}

impl From<&[u8]> for FieldName {
    fn from(value: &[u8]) -> Self {
        Self::from(value.to_vec())
    }
}

impl From<Vec<u8>> for FieldName {
    fn from(value: Vec<u8>) -> Self {
        let text = String::from_utf8_lossless(&value);

        Self {
            normalized: text.to_ascii_lowercase(),
            text: text.to_string(),
            raw: Some(value),
        }
    }
}

impl Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<FieldNameDe> for FieldName {
    fn from(original: FieldNameDe) -> Self {
        Self {
            normalized: original.text.to_ascii_lowercase(),
            text: original.text,
            raw: original.raw,
        }
    }
}

#[derive(Deserialize)]
struct FieldNameDe {
    text: String,

    raw: Option<Vec<u8>>,
}

/// Represents the value portion of a field.
///
/// The contents may be contain malformed or invalid sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    /// Value decoded.
    pub text: String,

    /// Value in the original encoded format.
    pub raw: Option<Vec<u8>>,
}

impl FieldValue {
    /// Creates a `FieldValue` with the given text and optional raw value.
    pub fn new(text: String, raw: Option<Vec<u8>>) -> Self {
        Self { text, raw }
    }

    /// Returns a string with potential invalid characters replaced.
    ///
    /// This is intended for debugging purposes.
    pub fn to_text_lossy(&self) -> String {
        self.text.replace(|c| c == '\r' || c == '\n', "\u{FFFD}")
    }
}

impl From<&str> for FieldValue {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        Self {
            text: value,
            raw: None,
        }
    }
}

impl From<&[u8]> for FieldValue {
    fn from(value: &[u8]) -> Self {
        Self::from(value.to_vec())
    }
}

impl From<Vec<u8>> for FieldValue {
    fn from(value: Vec<u8>) -> Self {
        Self {
            text: String::from_utf8_lossless(&value),
            raw: Some(value),
        }
    }
}

impl Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_get_convenience() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("k2", "v2");

        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
        assert!(map.contains_key("k1"));
        assert!(map.contains_key("k2"));
        assert!(!map.contains_key("k3"));
        assert_eq!(map.get_str("k1"), Some("v1"));
        assert_eq!(map.get_str("k2"), Some("v2"));
        assert_eq!(map.get_str("k3"), None);
    }

    #[test]
    fn test_header_map_uniques() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("k2", "v2");

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("k1").map(|v| v.text.as_ref()), Some("v1"));
        assert_eq!(map.get("k2").map(|v| v.text.as_ref()), Some("v2"));

        map.insert("k2", "hello world");

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("k1").map(|v| v.text.as_ref()), Some("v1"));
        assert_eq!(map.get("k2").map(|v| v.text.as_ref()), Some("hello world"));
    }

    #[test]
    fn test_header_map_duplicates() {
        let mut map = HeaderMap::new();

        map.append("k1", "v1");
        map.append("k2", "v2");
        map.append("k1", "v3");
        map.append("k2", "v4");

        assert_eq!(map.len(), 4);
        assert_eq!(map.get("k1").map(|v| v.text.as_ref()), Some("v1"));
        assert_eq!(map.get("k2").map(|v| v.text.as_ref()), Some("v2"));
        assert_eq!(
            map.get_all("k1")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["v1", "v3"]
        );
        assert_eq!(
            map.get_all("k2")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["v2", "v4"]
        );

        map.insert("k1", "hello world");

        assert_eq!(map.len(), 3);
        assert_eq!(
            map.get_all("k1")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["hello world"]
        );
        assert_eq!(
            map.get_all("k2")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["v2", "v4"]
        );
    }

    #[test]
    fn test_header_iter() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("k2", "v2");

        assert_eq!(map.len(), 2);

        let pairs = map.iter().cloned().collect::<Vec<FieldPair>>();

        assert_eq!(pairs[0].name.text, "k1");
        assert_eq!(pairs[0].value.text, "v1");
        assert_eq!(pairs[1].name.text, "k2");
        assert_eq!(pairs[1].value.text, "v2");
    }

    #[test]
    fn test_header_map_case_sensitivity() {
        let mut map = HeaderMap::new();

        map.insert("Hello-World", "v1");
        map.insert("hello-world", "v2");

        assert_eq!(map.len(), 1);
        assert_eq!(map.get("HELLO-WORLD").map(|v| v.text.as_ref()), Some("v2"));

        map.append("HELLO-world", "v3");

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get_all("hello-WORLD")
                .map(|v| v.text.to_string())
                .collect::<Vec<String>>(),
            vec!["v2", "v3"]
        );
    }

    #[test]
    fn test_display() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("k2", "v2");

        let result = map.to_string();

        assert_eq!(result, "k1: v1\r\nk2: v2\r\n");
    }

    #[test]
    fn test_reorder_front() {
        let mut map = HeaderMap::new();

        map.insert("k1", "v1");
        map.insert("Host", "example.com");
        map.append("Host", "example.net");
        map.reorder_front("host");

        let list = map
            .iter()
            .map(|pair| (pair.name.text.as_str(), pair.value.text.as_str()))
            .collect::<Vec<(&str, &str)>>();

        assert_eq!(list[0], ("Host", "example.com"));
        assert_eq!(list[1], ("Host", "example.net"));
        assert_eq!(list[2], ("k1", "v1"));
    }
}
