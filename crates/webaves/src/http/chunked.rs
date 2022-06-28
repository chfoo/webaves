//! Chunked transfer coding.

use std::io::{BufRead, Read, Take};

use crate::{
    header::{HeaderMap, HeaderParser},
    io::BufReadMoreExt,
    nomutil::NomParseError,
};

use super::HTTPError;

/// Manual decoder for a stream in chunked transfer coding.
pub struct ChunkedDecoder<R: BufRead> {
    stream: Option<R>,
    data_reader: Option<ChunkDataReader<R>>,
    state: DecoderState,
    buffer: Vec<u8>,
    buffer_limit: u64,
    chunk_length: u64,
    // chunk_amount_read: u64,
}

impl<R> ChunkedDecoder<R>
where
    R: BufRead,
{
    /// Creates a `ChunkedEncodingReader` with the given stream.
    pub fn new(stream: R) -> Self {
        Self {
            stream: Some(stream),
            data_reader: None,
            state: DecoderState::StartOfLine,
            buffer: Vec::new(),
            buffer_limit: 32768,
            chunk_length: 0,
            // chunk_amount_read: 0,
        }
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &R {
        self.stream
            .as_ref()
            .unwrap_or_else(|| self.data_reader.as_ref().unwrap().stream.get_ref())
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut R {
        self.stream
            .as_mut()
            .unwrap_or_else(|| self.data_reader.as_mut().unwrap().stream.get_mut())
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> R {
        self.stream
            .unwrap_or_else(|| self.data_reader.unwrap().stream.into_inner())
    }

    /// Starts reading a chunk.
    ///
    /// The caller must use [Self::read_data] next.
    ///
    /// Panics if called out of sequence.
    pub fn begin_chunk(&mut self) -> Result<ChunkMetadata, HTTPError> {
        tracing::debug!("begin_chunk");
        assert!(self.state == DecoderState::StartOfLine);
        self.buffer.clear();

        self.stream
            .as_mut()
            .unwrap()
            .read_limit_until(b'\n', &mut self.buffer, 4096)?;
        let metadata = parse_chunk_metadata(&self.buffer)?;
        self.chunk_length = metadata.length;

        self.state = DecoderState::EndOfLine;

        Ok(metadata)
    }

    /// Returns a reader for reading the chunk data.
    ///
    /// The caller must fully read until it returns no more data and then
    /// use [Self::end_chunk].
    ///
    /// Panics if called out of sequence.
    pub fn read_data(&mut self) -> &mut ChunkDataReader<R> {
        if self.stream.is_some() {
            self.set_up_chunk_data_reader();
        }

        self.data_reader.as_mut().unwrap()
    }

    fn set_up_chunk_data_reader(&mut self) {
        tracing::debug!(chunk_length = self.chunk_length, "read_data");
        assert!(self.state == DecoderState::EndOfLine);

        self.state = DecoderState::InBody;

        let stream = self.stream.take().unwrap().take(self.chunk_length);
        let reader = ChunkDataReader {
            stream,
            amount_read: 0,
        };

        self.data_reader = Some(reader);
    }

    /// Finishes reading a chunk.
    ///
    /// If the chunk size was 0, the caller must call [Self::read_trailer] next.
    /// Otherwise, the caller must use [Self::read_trailer].
    ///
    /// Panics if called out of sequence.
    pub fn end_chunk(&mut self) -> Result<(), HTTPError> {
        tracing::debug!("end_chunk");
        assert!(self.state == DecoderState::InBody);

        let data_reader = self.data_reader.take().unwrap();

        if data_reader.amount_read != self.chunk_length {
            return Err(HTTPError::UnexpectedEnd);
        }

        self.stream = Some(data_reader.stream.into_inner());

        if self.chunk_length == 0 {
            self.state = DecoderState::StartOfTrailer;
        } else {
            self.read_chunk_deliminator()?;
            self.state = DecoderState::StartOfLine;
        }

        Ok(())
    }

    fn read_chunk_deliminator(&mut self) -> Result<(), HTTPError> {
        tracing::debug!("read_chunk_deliminator");

        self.buffer.clear();
        self.stream
            .as_mut()
            .unwrap()
            .read_limit_until(b'\n', &mut self.buffer, 2)?;
        Ok(())
    }

    /// Finishes reading a stream.
    ///
    /// No more functions can be called after. Use [Self::into_inner] to get
    /// the wrapped stream back.
    ///
    /// Panics if called out of sequence.
    pub fn read_trailer(&mut self) -> Result<HeaderMap, HTTPError> {
        tracing::debug!("read_trailer");
        assert!(self.state == DecoderState::StartOfTrailer);

        self.buffer.clear();

        let stream = self.stream.as_mut().unwrap();
        crate::header::read_until_boundary(stream, &mut self.buffer, self.buffer_limit)?;

        let parser = HeaderParser::new();
        let header_map = parser
            .parse_header(crate::stringutil::trim_trailing_crlf(&self.buffer))
            .map_err(|error| HTTPError::InvalidEncoding {
                source: Some(Box::new(error)),
            })?;

        self.state = DecoderState::EndOfTrailer;

        Ok(header_map)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecoderState {
    StartOfLine,
    EndOfLine,
    InBody,
    StartOfTrailer,
    EndOfTrailer,
}

/// Reader for a chunk's data.
pub struct ChunkDataReader<R: BufRead> {
    stream: Take<R>,
    amount_read: u64,
}

impl<R: BufRead> Read for ChunkDataReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let amount = self.stream.read(buf)?;
        self.amount_read += amount as u64;
        Ok(amount)
    }
}

impl<R: BufRead> BufRead for ChunkDataReader<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.stream.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.stream.consume(amt)
    }
}

/// Decoded chunked coding size and extensions line.
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    /// Size of chunk data.
    pub length: u64,
    /// Chunk extensions.
    pub parameters: Vec<(String, String)>,
}

/// Parses chunked coding metadata line.
///
/// Input should contain the ending CRLF.
pub fn parse_chunk_metadata(line: &[u8]) -> Result<ChunkMetadata, HTTPError> {
    if let Ok(result) = super::pc::parse_chunk_metadata(line) {
        return Ok(ChunkMetadata {
            length: result.0,
            parameters: result.1,
        });
    };

    match super::pc::parse_chunk_metadata_fallback(line) {
        Ok(size) => Ok(ChunkMetadata {
            length: size,
            parameters: Vec::new(),
        }),
        Err(error) => Err(HTTPError::InvalidEncoding {
            source: Some(Box::new(NomParseError::from_nom(line, &error))),
        }),
    }
}

/// Reads and decodes a stream in chunked transfer coding.
pub struct ChunkedReader<R: BufRead> {
    inner: ChunkedDecoder<R>,
    state: ChunkedReaderState,
    chunk_size: u64,
    chunk_amount_read: u64,
}

impl<R: BufRead> ChunkedReader<R> {
    /// Creates a new `ChunkedReader` with the given stream.
    pub fn new(stream: R) -> Self {
        Self {
            inner: ChunkedDecoder::new(stream),
            state: ChunkedReaderState::Start,
            chunk_size: 0,
            chunk_amount_read: 0,
        }
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref()
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut()
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }

    fn remap_error(error: HTTPError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, error)
    }

    fn read_metadata(&mut self) -> std::io::Result<()> {
        let metadata = self.inner.begin_chunk().map_err(Self::remap_error)?;
        self.chunk_size = metadata.length;

        Ok(())
    }

    fn read_0_chunk_and_trailer(&mut self) -> std::io::Result<()> {
        let reader = self.inner.read_data();
        let mut temp = [0u8; 1];
        let _amount = reader.read(&mut temp)?;
        self.inner.end_chunk().map_err(Self::remap_error)?;
        self.inner.read_trailer().map_err(Self::remap_error)?;

        Ok(())
    }
}

impl<R: BufRead> Read for ChunkedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() || self.state == ChunkedReaderState::Finished {
            return Ok(0);
        }

        loop {
            if self.state == ChunkedReaderState::Start {
                self.read_metadata()?;

                if self.chunk_size == 0 {
                    self.read_0_chunk_and_trailer()?;
                    self.state = ChunkedReaderState::Finished;
                    return Ok(0);
                } else {
                    self.state = ChunkedReaderState::ReadingData;
                }
            };

            if self.state == ChunkedReaderState::ReadingData {
                let amount = self.inner.read_data().read(buf)?;

                self.chunk_amount_read += amount as u64;

                if amount == 0 && self.chunk_amount_read == self.chunk_size {
                    self.inner.end_chunk().map_err(Self::remap_error)?;
                    self.state = ChunkedReaderState::Start;
                } else {
                    return Ok(amount);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkedReaderState {
    Start,
    ReadingData,
    Finished,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_decoder() {
        let body = Cursor::new(b"3\r\nabc\r\n5\r\nhello\r\n0\r\nk1:v2\r\n\r\n");
        let mut reader = ChunkedDecoder::new(body);

        fn read_chunk<R: BufRead>(reader: &mut ChunkedDecoder<R>, expected: &[u8]) {
            let mut buffer = Vec::new();
            let metadata = reader.begin_chunk().unwrap();
            assert_eq!(metadata.length, expected.len() as u64);

            let data_reader = reader.read_data();
            data_reader.read_to_end(&mut buffer).unwrap();
            assert_eq!(buffer, expected);

            reader.end_chunk().unwrap();
        }

        read_chunk(&mut reader, b"abc");
        read_chunk(&mut reader, b"hello");
        read_chunk(&mut reader, b"");

        reader.read_trailer().unwrap();
    }

    #[test]
    fn test_parse_chunk_metadata() {
        let metadata = parse_chunk_metadata(b"0a\n").unwrap();
        assert_eq!(metadata.length, 10);

        let metadata = parse_chunk_metadata(b"0a;k1=v1\n").unwrap();
        assert_eq!(metadata.length, 10);
        assert_eq!(metadata.parameters[0].0, "k1");
        assert_eq!(metadata.parameters[0].1, "v1");
    }

    #[test]
    fn test_reader() {
        let body = Cursor::new(b"3\r\nabc\r\n5\r\nhello\r\n0\r\nk1:v2\r\n\r\n");
        let mut reader = ChunkedReader::new(body);

        let mut output = Vec::new();
        reader.read_to_end(&mut output).unwrap();

        assert_eq!(output, b"abchello");
    }
}
