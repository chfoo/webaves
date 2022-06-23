/// Additional character classes.
pub trait HeaderByteExt {
    /// Returns whether the octet is valid as a "token" character.
    fn is_token(&self) -> bool;

    /// Returns whether the octet is classified as "obs-text".
    fn is_obs_text(&self) -> bool;
}

impl HeaderByteExt for u8 {
    fn is_token(&self) -> bool {
        self.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(self)
    }

    fn is_obs_text(&self) -> bool {
        *self >= 0x80
    }
}

pub fn cut_start_line(buf: &[u8]) -> (&[u8], &[u8]) {
    let index = buf
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(buf.len() - 1);
    buf.split_at(index + 1)
}

pub fn trim_trailing_newline(buf: &[u8]) -> &[u8] {
    if buf.ends_with(b"\r\n") {
        &buf[0..buf.len() - 2]
    } else if buf.ends_with(b"\n") {
        &buf[0..buf.len() - 1]
    } else {
        buf
    }
}
