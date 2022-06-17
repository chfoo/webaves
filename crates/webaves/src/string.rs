//! Lossless UTF-8 string encoding/decoding.
//!
//! This module includes UTF-8 string decoding that preserves invalid UTF-8 byte sequences.
//!
//! Invalid bytes are replaced by two code points: replacement character (U+FFFD)
//! followed by a code point from U+105500 to U+1055FF representing the invalid byte.
//!
//! Input replacement character (U+FFFD) is replaced by two code points: U+FFFD followed by U+105600.

/// Decode UTF-8 bytes to string with lossless scheme.
pub fn decode(input: &[u8], output: &mut String) {
    if input.is_ascii() {
        output.push_str(&String::from_utf8_lossy(input));
        return;
    }

    let mut decoder = utf8::BufReadDecoder::new(input);

    while let Some(result) = decoder.next_strict() {
        match result {
            Ok(content) => {
                for char in content.chars() {
                    if char == char::REPLACEMENT_CHARACTER {
                        output.push(char::REPLACEMENT_CHARACTER);
                        output.push('\u{105600}');
                    } else {
                        output.push(char);
                    }
                }
            }
            Err(error) => match error {
                utf8::BufReadDecoderError::InvalidByteSequence(seq) => {
                    for b in seq {
                        output.push(char::REPLACEMENT_CHARACTER);
                        output.push(char::from_u32(0x105500 | *b as u32).unwrap());
                    }
                }
                utf8::BufReadDecoderError::Io(error) => panic!("{:?}", error),
            },
        }
    }
}

/// Encode string with lossless scheme to UTF-8 bytes.
pub fn encode(input: &str, output: &mut Vec<u8>) {
    const REPLACEMENT_CHARACTER_UTF8: [u8; 3] = [0xEF, 0xBF, 0xBD];

    if input.is_ascii() {
        output.extend_from_slice(input.as_bytes());
        return;
    }

    let mut escape = false;
    let mut buffer = [0u8; 4];
    let count = input.chars().count();

    for (index, c) in input.chars().enumerate() {
        if c == char::REPLACEMENT_CHARACTER && index != count - 1 {
            escape = true;
        } else if escape {
            if ('\u{105500}'..='\u{1055FF}').contains(&c) {
                output.push(c as u8);
            } else if c == '\u{105600}' {
                output.extend_from_slice(&REPLACEMENT_CHARACTER_UTF8);
            } else {
                output.extend_from_slice(&REPLACEMENT_CHARACTER_UTF8);
                output.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            }

            escape = false;
        } else {
            output.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
        }
    }
}

/// Convenience trait for string conversion.
pub trait StringLosslessExt {
    /// Decode UTF-8 bytes to string with lossless scheme.
    fn from_utf8_lossless(input: &[u8]) -> String;

    /// Encode string with lossless scheme to UTF-8 bytes.
    fn to_utf8_lossless(&self) -> Vec<u8>;
}

impl StringLosslessExt for String {
    fn from_utf8_lossless(input: &[u8]) -> String {
        let mut output = String::with_capacity(input.len());
        decode(input, &mut output);

        output
    }

    fn to_utf8_lossless(&self) -> Vec<u8> {
        let mut output = Vec::new();
        encode(self, &mut output);

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let mut buffer = String::new();

        decode(
            b"abc\xef\
            \xfe\
            \xEF\xBF\xBDefg\
            \xEF\xBF\xBD",
            &mut buffer,
        );

        assert_eq!(
            buffer,
            "abc\u{FFFD}\u{1055ef}\
            \u{FFFD}\u{1055fe}\
            \u{FFFD}\u{105600}efg\
            \u{FFFD}\u{105600}"
        );
    }

    #[test]
    fn test_decode_ascii() {
        let mut buffer = String::new();

        decode(b"hello", &mut buffer);

        assert_eq!(buffer, "hello");
    }

    #[test]
    fn test_encode() {
        let mut buffer = Vec::new();

        encode(
            "abc\u{FFFD}\u{105512}\
            \u{FFFD}\u{1055fe}\
            \u{FFFD}\u{105600}\
            \u{FFFD}efg\
            \u{FFFD}",
            &mut buffer,
        );

        assert_eq!(
            buffer,
            b"abc\x12\
            \xfe\
            \xEF\xBF\xBD\
            \xEF\xBF\xBDefg\
            \xEF\xBF\xBD"
        );
    }

    #[test]
    fn test_encode_ascii() {
        let mut buffer = Vec::new();

        encode("hello", &mut buffer);

        assert_eq!(buffer, b"hello");
    }
}
