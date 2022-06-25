/// Reads bytes into `output` until the header boundary has been found.
///
/// This function will read bytes from `input` and appends them to `output`,
/// stopping when the CRLF (or LF) separating the fields and message body has been
/// found. The output will include the CRLF (or LF) separator.
///
/// Returns the number of bytes read.
///
/// Returns an error if the boundary has not been found due to early end of
/// input or the input exceeds the size given by `limit`.
pub fn read_until_boundary<R>(input: R, output: &mut Vec<u8>, limit: u64) -> std::io::Result<u64>
where
    R: std::io::BufRead,
{
    use std::io::{BufRead, Error, ErrorKind};

    let mut input = input.take(limit);
    let mut total_amount = 0u64;

    loop {
        let output_position = output.len();
        let amount = input.read_until(b'\n', output)?;
        total_amount += amount as u64;

        let line = &output[output_position..];

        if line.is_empty() {
            if total_amount < limit {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "no CRLF boundary found",
                ));
            } else {
                return Err(Error::new(ErrorKind::InvalidData, "input header too long"));
            }
        } else if b"\r\n".contains(&line[0]) {
            break;
        }
    }

    Ok(total_amount)
}

/// Reads bytes into `output` until the header boundary has been found.
///
/// See [read_until_boundary] for full description.
pub async fn read_async_until_boundary<R>(
    input: R,
    output: &mut Vec<u8>,
    limit: u64,
) -> std::io::Result<u64>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    use std::io::{Error, ErrorKind};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt};

    let mut input = input.take(limit);
    let mut total_amount = 0u64;

    loop {
        let output_position = output.len();
        let amount = input.read_until(b'\n', output).await?;
        total_amount += amount as u64;

        let line = &output[output_position..];

        if line.is_empty() {
            if total_amount < limit {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "no CRLF boundary found",
                ));
            } else {
                return Err(Error::new(ErrorKind::InvalidData, "input header too long"));
            }
        } else if b"\r\n".contains(&line[0]) {
            break;
        }
    }

    Ok(total_amount)
}

/// Trims the trailing CRLF or LF.
///
/// Example:
///
/// ```rust
/// # use webaves::header::trim_trailing_crlf;
/// assert_eq!(trim_trailing_crlf(b"abc\r\n\r\n"), b"abc\r\n");
/// assert_eq!(trim_trailing_crlf(b"abc\r\n"), b"abc");
/// assert_eq!(trim_trailing_crlf(b"abc\n\n"), b"abc\n");
/// assert_eq!(trim_trailing_crlf(b"abc\n"), b"abc");
/// ```
pub fn trim_trailing_crlf(buf: &[u8]) -> &[u8] {
    if buf.ends_with(b"\r\n") {
        &buf[0..buf.len() - 2]
    } else if buf.ends_with(b"\n") {
        &buf[0..buf.len() - 1]
    } else {
        buf
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_read_until_boundary_crlf() {
        let mut input = Cursor::new(b"a\r\nb\r\n\r\nc");
        let mut output = Vec::new();
        let count = read_until_boundary(&mut input, &mut output, 9999).unwrap();

        assert_eq!(count, 8);
        assert_eq!(&output, b"a\r\nb\r\n\r\n");
        assert_eq!(input.position(), 8);
    }

    #[test]
    fn test_read_until_boundary_lf() {
        let mut input = Cursor::new(b"a\nb\n\nc");
        let mut output = Vec::new();
        let count = read_until_boundary(&mut input, &mut output, 9999).unwrap();

        assert_eq!(count, 5);
        assert_eq!(&output, b"a\nb\n\n");
        assert_eq!(input.position(), 5);
    }

    #[test]
    fn test_read_until_boundary_eof() {
        let mut input = Cursor::new(b"a\r\nb\r\n");
        let mut output = Vec::new();
        let result = read_until_boundary(&mut input, &mut output, 9999);

        assert!(result.is_err());
    }

    #[test]
    fn test_read_until_boundary_limit() {
        let mut input = Cursor::new(b"aaaaa\r\nbbbbb\r\n\r\nccccc");
        let mut output = Vec::new();
        let result = read_until_boundary(&mut input, &mut output, 7);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_async_until_boundary_crlf() {
        let mut input = Cursor::new(b"a\r\nb\r\n\r\nc");
        let mut output = Vec::new();
        let count = read_async_until_boundary(&mut input, &mut output, 9999)
            .await
            .unwrap();

        assert_eq!(count, 8);
        assert_eq!(&output, b"a\r\nb\r\n\r\n");
        assert_eq!(input.position(), 8);
    }

    #[tokio::test]
    async fn test_read_async_until_boundary_lf() {
        let mut input = Cursor::new(b"a\nb\n\nc");
        let mut output = Vec::new();
        let count = read_async_until_boundary(&mut input, &mut output, 9999)
            .await
            .unwrap();

        assert_eq!(count, 5);
        assert_eq!(&output, b"a\nb\n\n");
        assert_eq!(input.position(), 5);
    }

    #[tokio::test]
    async fn test_read_async_until_boundary_eof() {
        let mut input = Cursor::new(b"a\r\nb\r\n");
        let mut output = Vec::new();
        let result = read_async_until_boundary(&mut input, &mut output, 9999).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_async_until_boundary_limit() {
        let mut input = Cursor::new(b"aaaaa\r\nbbbbb\r\n\r\nccccc");
        let mut output = Vec::new();
        let result = read_async_until_boundary(&mut input, &mut output, 7).await;

        assert!(result.is_err());
    }

    #[test]
    fn text_trim_trailing_crlf() {
        assert_eq!(trim_trailing_crlf(b"abc\r\n\r\n"), b"abc\r\n");
        assert_eq!(trim_trailing_crlf(b"abc\r\n"), b"abc");
        assert_eq!(trim_trailing_crlf(b"abc\n\n"), b"abc\n");
        assert_eq!(trim_trailing_crlf(b"abc\n"), b"abc");
    }
}
