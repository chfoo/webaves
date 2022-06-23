//! Various string and character tools.

/// Additional character classes.
pub trait CharClassExt {
    /// Returns whether the octet is valid as a "token" character.
    ///
    /// Any ASCII character except controls and separators.
    fn is_token(&self) -> bool;

    /// Returns whether the octet is a "separators" character.
    ///
    /// `( ) < > @ , ; : Backslash " / [ ] ? = { } Space Tab`
    fn is_separator(&self) -> bool;

    /// Returns whether the octet is valid as a printable or opaque character.
    ///
    /// Any octet except controls but including WS (Space Tab).
    fn is_text_ws(&self) -> bool;

    /// Returns whether the octet is valid as a classic "TEXT" character.
    ///
    /// Any octet except controls but including LWS.
    fn is_text_lws(&self) -> bool;

    /// Returns whether the octet is valid as a whitespace character.
    ///
    /// `Space Tab`
    fn is_ws(&self) -> bool;

    /// Returns whether the octet is valid as a linear whitespace "LWS" character.
    ///
    /// `CR LF Space Tab`
    fn is_lws(&self) -> bool;

    /// Returns the number of bytes in a UTF-8 sequence.
    ///
    /// - If 1, then the octet encodes itself.
    /// - If 2, then the octet encodes itself and 1 following octet.
    /// - If 3, then the octet encodes itself and 2 following octets.
    /// - If 4, then the octet encodes itself and 3 following octets.
    /// - Otherwise, 0, invalid encoding.
    fn sequence_length(&self) -> u32;
}

impl CharClassExt for u8 {
    fn is_token(&self) -> bool {
        self.is_ascii() && !self.is_ascii_control() && !self.is_separator()
    }

    fn is_separator(&self) -> bool {
        b"()<>@,;:\\\"/[]?={} \t".contains(self)
    }

    fn is_ws(&self) -> bool {
        b"\t ".contains(self)
    }

    fn is_text_ws(&self) -> bool {
        !self.is_ascii_control() || b" \t".contains(self)
    }

    fn is_text_lws(&self) -> bool {
        !self.is_ascii_control() || b"\r\n \t".contains(self)
    }

    fn is_lws(&self) -> bool {
        b"\r\n \t".contains(self)
    }

    fn sequence_length(&self) -> u32 {
        match self.leading_ones() {
            0 => 1,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_byte_ext() {
        assert!(b'a'.is_token());
        assert!(!b'\n'.is_token());
        assert!(!b':'.is_token());

        assert!(b':'.is_separator());
        assert!(!b'a'.is_separator());

        assert!(b'a'.is_text_lws());
        assert!(b'\t'.is_text_lws());
        assert!(b'\n'.is_text_lws());
        assert!(!b'\x00'.is_text_lws());

        assert!(b'\t'.is_lws());
        assert!(!b'a'.is_lws());

        assert_eq!(b'a'.sequence_length(), 1);
        assert_eq!(b'\x80'.sequence_length(), 1);
        assert_eq!(b'\xC4'.sequence_length(), 2);
        assert_eq!(b'\xE3'.sequence_length(), 3);
        assert_eq!(b'\xF0'.sequence_length(), 4);
        assert_eq!(b'\xFF'.sequence_length(), 0);
    }
}
