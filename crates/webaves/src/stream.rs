//! Stream helpers.

use std::io::{BufRead, Read};

pub struct PeekReader<R: Read> {
    inner: R,
    buf: Vec<u8>,
}

impl<R: Read> PeekReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            inner: reader,
            buf: Vec::new(),
        }
    }

    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    #[allow(dead_code)]
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    /// Read exactly `amount` number of bytes without consuming it.
    ///
    /// This function reads bytes from the wrapped [Read], appends them to an
    /// internal buffer, and returns a slice to the bytes that was read.
    ///
    /// Calls to [Read:read] will return bytes from the internal buffer,
    /// removing the corresponding bytes until the internal buffer is empty.
    /// Once the buffer is empty, reading will call directly the wrapped object.
    pub fn peek(&mut self, amount: usize) -> std::io::Result<&[u8]> {
        let original_buf_len = self.buf.len();
        self.buf.resize(original_buf_len + amount, 0);
        self.inner
            .read_exact(&mut self.buf[original_buf_len..original_buf_len + amount])?;

        Ok(&self.buf[original_buf_len..original_buf_len + amount])
    }
}

impl<R: Read> Read for PeekReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.buf.is_empty() {
            self.inner.read(buf)
        } else {
            let len = buf.len().min(self.buf.len());
            buf[0..len].copy_from_slice(&self.buf[0..len]);
            self.buf.copy_within(len.., 0);
            self.buf.truncate(self.buf.len() - len);

            Ok(len)
        }
    }
}

pub trait CountRead {
    /// Returns the total number of bytes that have been read by the caller.
    fn read_count(&self) -> u64;
}

pub struct CountReader<R: Read> {
    inner: R,
    count: u64,
}

impl<R: Read> CountReader<R> {
    #[allow(dead_code)]
    pub fn new(inner: R) -> Self {
        Self { inner, count: 0 }
    }

    #[allow(dead_code)]
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> CountRead for CountReader<R> {
    fn read_count(&self) -> u64 {
        self.count
    }
}

impl<R: Read> Read for CountReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.inner.read(buf)?;
        self.count += size as u64;
        Ok(size)
    }
}

pub struct CountBufReader<R: BufRead> {
    inner: R,
    count: u64,
}

impl<R: BufRead> CountBufReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner, count: 0 }
    }

    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: BufRead> Read for CountBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.inner.read(buf)?;
        Ok(size)
    }
}

impl<R: BufRead> BufRead for CountBufReader<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.count += amt as u64;
        self.inner.consume(amt)
    }
}

impl<R: BufRead> CountRead for CountBufReader<R> {
    fn read_count(&self) -> u64 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_peek_reader() {
        let source = Cursor::new(b"0123456789abcdef");
        let mut reader = PeekReader::new(source);
        let mut output = Vec::new();

        output.resize(2, 0);
        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, b"01");
        assert_eq!(reader.buffer(), b"");

        assert_eq!(reader.peek(2).unwrap(), b"23");
        assert_eq!(reader.buffer(), b"23");

        output.resize(1, 0);
        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, b"2");
        assert_eq!(reader.buffer(), b"3");

        output.resize(2, 0);
        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, b"34");
        assert_eq!(reader.buffer(), b"");

        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, b"56");

        assert_eq!(reader.peek(2).unwrap(), b"78");
        assert_eq!(reader.buffer(), b"78");
        assert_eq!(reader.peek(2).unwrap(), b"9a");
        assert_eq!(reader.buffer(), b"789a");

        output.resize(5, 0);
        reader.read_exact(&mut output).unwrap();
        assert_eq!(output, b"789ab");
        assert_eq!(reader.buffer(), b"");
    }

    #[test]
    fn test_count_reader() {
        let source = Cursor::new(b"0123456789abcdef");
        let mut reader = CountReader::new(source);
        let mut output = Vec::new();

        output.resize(5, 0);
        reader.read_exact(&mut output).unwrap();
        assert_eq!(5, reader.read_count());

        output.resize(6, 0);
        reader.read_exact(&mut output).unwrap();
        assert_eq!(11, reader.read_count());
    }

    #[test]
    fn test_count_buf_reader() {
        let source = Cursor::new(b"0123456789abcdef");
        let mut reader = CountBufReader::new(source);
        let mut output = Vec::new();

        output.resize(5, 0);
        reader.read_exact(&mut output).unwrap();
        assert_eq!(5, reader.read_count());

        output.resize(6, 0);
        reader.read_exact(&mut output).unwrap();
        assert_eq!(11, reader.read_count());
    }
}
