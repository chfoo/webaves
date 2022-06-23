pub fn cut_start_line(buf: &[u8]) -> (&[u8], &[u8]) {
    let index = buf
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(buf.len() - 1);
    buf.split_at(index + 1)
}

#[allow(dead_code)]
pub fn trim_trailing_newline(buf: &[u8]) -> &[u8] {
    if buf.ends_with(b"\r\n") {
        &buf[0..buf.len() - 2]
    } else if buf.ends_with(b"\n") {
        &buf[0..buf.len() - 1]
    } else {
        buf
    }
}
