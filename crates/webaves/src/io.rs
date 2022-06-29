//! IO helpers.

use std::io::{BufRead, Error, ErrorKind, Read, Result};

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

/// Read data without consuming it.
pub trait PeekRead {
    /// Returns data from the stream without advancing the stream position.
    ///
    /// At most one read call is made to fill the buffer and returns a slice
    /// to the buffer. The length of the slice may be smaller than requested.
    fn peek(&mut self, amount: usize) -> Result<&[u8]>;

    /// Returns data from the stream without advancing the stream position.
    ///
    /// This function is similar to [Self:peek] except the length of the slice
    /// returned will be equal to `amount`. Returns an error if EOF.
    fn peek_exact(&mut self, amount: usize) -> Result<&[u8]> {
        let mut prev_buf_len = 0;

        loop {
            let buffer = self.peek(amount)?;

            if buffer.len() >= amount {
                break;
            } else if prev_buf_len == buffer.len() {
                return Err(ErrorKind::UnexpectedEof.into());
            }

            prev_buf_len = buffer.len();
        }

        self.peek(amount)
    }
}

/// Count number of bytes read.
pub trait CountRead {
    /// Returns the number of bytes read from this stream.
    ///
    /// The value represents the number of bytes marked as consumed and not
    /// bytes stored in internal buffers. If the stream is seekable, seeking
    /// does not affect this value.
    fn read_count(&self) -> u64;
}

/// Count number of bytes from a source stream.
///
/// This trait is for reader objects that wrap another stream and transform
/// data such as a decoders.
pub trait SourceCountRead {
    /// Returns the number of bytes read by this object from the source stream.
    fn source_read_count(&self) -> u64;
}

/// Buffered reader various features implemented.
pub struct ComboReader<R: Read> {
    stream: R,
    buf: Vec<u8>,
    buf_len_threshold: usize,
    read_count: u64,
    source_read_count: u64,
}

impl<R: Read> ComboReader<R> {
    /// Creates a reader with the given stream.
    pub fn new(reader: R) -> Self {
        Self {
            stream: reader,
            buf: Vec::new(),
            buf_len_threshold: 4096,
            read_count: 0,
            source_read_count: 0,
        }
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &R {
        &self.stream
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.stream
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> R {
        self.stream
    }

    /// Returns a reference to the internal buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    fn fill_buf_impl(&mut self, amount: usize) -> Result<()> {
        if self.buf.len() < amount {
            let offset = self.buf.len();
            self.buf.resize(offset + self.buf_len_threshold, 0);
            let amount = self.stream.read(&mut self.buf[offset..])?;
            self.buf.truncate(offset + amount);

            self.source_read_count += amount as u64;
        }

        Ok(())
    }

    fn shift_buf(&mut self, amount: usize) {
        self.buf.copy_within(amount.., 0);
        self.buf.truncate(self.buf.len() - amount);
    }
}

impl<R: Read> Read for ComboReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.buf.is_empty() {
            let amount = self.buf.len().min(buf.len());
            (&mut buf[0..amount]).copy_from_slice(&self.buf[0..amount]);
            self.shift_buf(amount);

            self.read_count += amount as u64;

            Ok(amount)
        } else if buf.len() >= self.buf_len_threshold {
            debug_assert!(self.buf.is_empty());

            let amount = self.stream.read(buf)?;

            self.source_read_count += amount as u64;
            self.read_count += amount as u64;

            Ok(amount)
        } else {
            debug_assert!(self.buf.is_empty());

            self.fill_buf()?;
            let amount = buf.len().min(self.buf.len());
            (&mut buf[0..amount]).copy_from_slice(&self.buf[0..amount]);
            self.consume(amount);

            Ok(amount)
        }
    }
}

impl<R: Read> BufRead for ComboReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.fill_buf_impl(self.buf_len_threshold)?;

        Ok(&self.buf)
    }

    fn consume(&mut self, amount: usize) {
        let amount = self.buf.len().min(amount);
        self.shift_buf(amount);

        self.read_count += amount as u64;
    }
}

impl<R: Read> PeekRead for ComboReader<R> {
    fn peek(&mut self, amount: usize) -> Result<&[u8]> {
        self.fill_buf_impl(amount)?;

        let amount = amount.min(self.buf.len());

        Ok(&self.buf[0..amount])
    }
}

impl<R: Read> CountRead for ComboReader<R> {
    fn read_count(&self) -> u64 {
        self.read_count
    }
}

impl<R: Read> SourceCountRead for ComboReader<R> {
    fn source_read_count(&self) -> u64 {
        self.source_read_count
    }
}

#[cfg(test)]
mod tests_sync {
    use crate::io::{BufReadMoreExt, CountRead, SourceCountRead};
    use std::io::{BufRead, Cursor, Read};

    use super::{PeekRead, ComboReader};

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

    #[test]
    fn test_combo_reader_read() {
        let input = Cursor::new(b"0123456789abcdef");
        let mut reader = ComboReader::new(input);
        let mut output = Vec::new();

        output.resize(2, 0);
        let amount = reader.read(&mut output).unwrap();
        assert_eq!(amount, 2);
        assert_eq!(output, b"01");
        assert_eq!(reader.buffer(), b"23456789abcdef");
        assert_eq!(reader.read_count(), 2);
        assert_eq!(reader.source_read_count(), 16);

        output.resize(4, 0);
        let amount = reader.read(&mut output).unwrap();
        assert_eq!(amount, 4);
        assert_eq!(output, b"2345");
        assert_eq!(reader.buffer(), b"6789abcdef");
        assert_eq!(reader.read_count(), 6);
        assert_eq!(reader.source_read_count(), 16);

        output.resize(100, 0);
        let amount = reader.read(&mut output).unwrap();
        assert_eq!(amount, 10);
        assert_eq!(&output[0..10], b"6789abcdef");
        assert_eq!(reader.buffer(), b"");
        assert_eq!(reader.read_count(), 16);
        assert_eq!(reader.source_read_count(), 16);

        let amount = reader.read(&mut output).unwrap();
        assert_eq!(amount, 0);
        assert_eq!(reader.buffer(), b"");
        assert_eq!(reader.read_count(), 16);
        assert_eq!(reader.source_read_count(), 16);
    }

    #[test]
    fn test_combo_reader_bufread() {
        let input = Cursor::new(b"0123456789abcdef");
        let mut reader = ComboReader::new(input);

        let buffer = reader.fill_buf().unwrap();
        assert_eq!(buffer, b"0123456789abcdef");
        assert_eq!(reader.read_count(), 0);
        assert_eq!(reader.source_read_count(), 16);

        reader.consume(4);
        assert_eq!(reader.buffer(), b"456789abcdef");
        assert_eq!(reader.read_count(), 4);
        assert_eq!(reader.source_read_count(), 16);

        let buffer = reader.fill_buf().unwrap();
        assert_eq!(buffer, b"456789abcdef");
        assert_eq!(reader.read_count(), 4);
        assert_eq!(reader.source_read_count(), 16);

        reader.consume(12);
        assert_eq!(reader.buffer(), b"");
        assert_eq!(reader.read_count(), 16);
        assert_eq!(reader.source_read_count(), 16);
    }

    #[test]
    fn test_combo_reader_peek() {
        let input = Cursor::new(b"0123456789abcdef");
        let mut reader = ComboReader::new(input);

        let output = reader.peek(4).unwrap();
        assert_eq!(output, b"0123");
        let output = reader.peek_exact(4).unwrap();
        assert_eq!(output, b"0123");

        let mut output = Vec::new();
        output.resize(6, 0);

        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, b"012345");

        let output = reader.peek(4).unwrap();
        assert_eq!(output, b"6789");
        let output = reader.peek_exact(4).unwrap();
        assert_eq!(output, b"6789");

        let mut output = Vec::new();
        output.resize(6, 0);

        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, b"6789ab");

        let result = reader.peek_exact(9999);
        assert!(result.is_err());
    }

    #[test]
    fn test_combo_reader_big_read() {
        let mut input = Vec::new();

        for _ in 0..5000 {
            input.extend_from_slice(b"0123456789abcdef");
        }

        let input = Cursor::new(input);
        let mut reader = ComboReader::new(input);

        let mut output = Vec::new();
        output.resize(5000, 0);

        let amount = reader.read(&mut output).unwrap();
        assert_eq!(amount, 5000);
        assert_eq!(reader.read_count(), 5000);
        assert_eq!(reader.source_read_count(), 5000);
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
        let count = input
            .read_limit_until(b'\n', &mut output, 9999)
            .await
            .unwrap();

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
