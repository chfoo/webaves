//! IO helpers.

use std::io::{BufRead, Error, ErrorKind, Result};

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

/// Extension trait for [std::io::BufRead].
pub trait BufReadMoreExt {
    /// Reads bytes into `buf` until the delimiter `byte` or EOF is reached.
    ///
    /// This function is similar to [std::io::BufRead::read_until].
    /// In addition, this function returns an error when the number of bytes
    /// read equals `limit` and the deliminator has not been reached.
    fn read_limit_until(&mut self, byte: u8, buf: &mut Vec<u8>, limit: u64) -> Result<usize>;
}

/// Extension trait for [tokio::io::AsyncBufRead].
#[async_trait::async_trait]
pub trait AsyncBufReadMoreExt {
    /// Reads bytes into `buf` until the delimiter `byte` or EOF is reached.
    ///
    /// Equivalent to:
    ///
    /// ```ignore
    /// async fn read_limit_until(&mut self, byte: u8, buf: &mut Vec<u8>, limit: u64) -> Result<usize>;
    /// ```
    ///
    /// This function is similar to [tokio::io::AsyncBufReadExt::read_until].
    /// In addition, this function returns an error when the number of bytes
    /// read equals `limit` and the deliminator has not been reached.
    async fn read_limit_until(&mut self, byte: u8, buf: &mut Vec<u8>, limit: u64) -> Result<usize>;
}

impl<R: BufRead> BufReadMoreExt for R {
    fn read_limit_until(&mut self, byte: u8, buf: &mut Vec<u8>, limit: u64) -> Result<usize> {
        // Compiler won't use Take<&mut R> in trait here so it's in a separate function.
        read_limit_until(self, byte, buf, limit)
    }
}

fn read_limit_until<R: BufRead>(
    stream: R,
    byte: u8,
    buf: &mut Vec<u8>,
    limit: u64,
) -> Result<usize> {
    let mut stream = stream.take(limit);
    let amount = stream.read_until(byte, buf)?;

    if amount as u64 == limit && !buf.ends_with(&[byte]) {
        return Err(Error::new(ErrorKind::InvalidData, "line too long"));
    }

    Ok(amount)
}

#[async_trait::async_trait]
impl<R: AsyncBufRead + Send + Unpin> AsyncBufReadMoreExt for R {
    async fn read_limit_until(&mut self, byte: u8, buf: &mut Vec<u8>, limit: u64) -> Result<usize> {
        let mut stream = self.take(limit);
        let amount = stream.read_until(byte, buf).await?;

        if amount as u64 == limit && !buf.ends_with(&[byte]) {
            return Err(Error::new(ErrorKind::InvalidData, "line too long"));
        }

        Ok(amount)
    }
}

#[cfg(test)]
mod tests_sync {
    use crate::io::BufReadMoreExt;
    use std::io::Cursor;

    #[test]
    fn test_read_limit_until() {
        let mut input = Cursor::new(b"a\r\nb\r\n\r\nc");
        let mut output = Vec::new();
        let count = input.read_limit_until(b'\n', &mut output, 9999).unwrap();

        assert_eq!(count, 3);
        assert_eq!(&output, b"a\r\n");
        assert_eq!(input.position(), 3);
    }

    #[test]
    fn test_read_limit_until_eof() {
        let mut input = Cursor::new(b"abc");
        let mut output = Vec::new();
        let count = input.read_limit_until(b'\n', &mut output, 9999).unwrap();

        assert_eq!(count, 3);
        assert_eq!(&output, b"abc");
        assert_eq!(input.position(), 3);
    }

    #[test]
    fn test_read_limit_until_limit() {
        let mut input = Cursor::new(b"aaaaabbbbbccccc");
        let mut output = Vec::new();
        let result = input.read_limit_until(b'\n', &mut output, 7);

        assert!(result.is_err());
    }
}
#[cfg(test)]
mod tests_async {
    use crate::io::AsyncBufReadMoreExt;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_read_limit_until() {
        let mut input = Cursor::new(b"a\r\nb\r\n\r\nc");
        let mut output = Vec::new();
        let count = input
            .read_limit_until(b'\n', &mut output, 9999)
            .await
            .unwrap();

        assert_eq!(count, 3);
        assert_eq!(&output, b"a\r\n");
        assert_eq!(input.position(), 3);
    }

    #[tokio::test]
    async fn test_read_limit_until_eof() {
        let mut input = Cursor::new(b"abc");
        let mut output = Vec::new();
        let count = input.read_limit_until(b'\n', &mut output, 9999).await.unwrap();

        assert_eq!(count, 3);
        assert_eq!(&output, b"abc");
        assert_eq!(input.position(), 3);
    }

    #[tokio::test]
    async fn test_read_limit_until_limit() {
        let mut input = Cursor::new(b"aaaaabbbbbccccc");
        let mut output = Vec::new();
        let result = input.read_limit_until(b'\n', &mut output, 7).await;

        assert!(result.is_err());
    }
}
