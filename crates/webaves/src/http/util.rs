/// Splits a header into the first line and remainder.
pub fn cut_start_line(buf: &[u8]) -> (&[u8], &[u8]) {
    let index = buf
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(buf.len() - 1);
    buf.split_at(index + 1)
}
