//! Chunked transfer coding.

use std::io::{BufRead, Read, Take};

use crate::{
    header::{HeaderMap, HeaderParser},
    io::BufReadMoreExt,
    nomutil::NomParseError,
};

use super::HTTPError;

/// Reader for a stream with chunked encoding.
pub struct ChunkedEncodingReader<R: BufRead> {
    stream: R,
    state: ChunkedReaderState,
    buffer: Vec<u8>,
    buffer_limit: u64,
    chunk_length: u64,
    chunk_amount_read: u64,
}

impl<R> ChunkedEncodingReader<R>
where
    R: BufRead,
{
    /// Creates a `ChunkedEncodingReader` with the given stream.
    pub fn new(stream: R) -> Self {
        Self {
            stream,
            state: ChunkedReaderState::StartOfLine,
            buffer: Vec::new(),
            buffer_limit: 32768,
            chunk_length: 0,
            chunk_amount_read: 0,
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

    /// Starts reading a chunk.
    ///
    /// The caller must use [Self::read_data] next.
    ///
    /// Panics if called out of sequence.
    pub fn begin_chunk(&mut self) -> Result<ChunkMetadata, HTTPError> {
        tracing::debug!("begin_chunk");
        assert!(self.state == ChunkedReaderState::StartOfLine);
        self.buffer.clear();

        self.stream
            .read_limit_until(b'\n', &mut self.buffer, 4096)?;
        let metadata = parse_chunk_metadata(&self.buffer)?;
        self.chunk_length = metadata.length;
        self.chunk_amount_read = 0;

        self.state = ChunkedReaderState::EndOfLine;

        Ok(metadata)
    }

    /// Returns a reader for reading the chunk data.
    ///
    /// The caller must fully read until it returns no more data and then
    /// use [Self::end_chunk].
    ///
    /// Panics if called out of sequence.
    pub fn read_data(&mut self) -> ChunkDataReader<'_, R> {
        tracing::debug!(chunk_length = self.chunk_length, "read_data");
        assert!(self.state == ChunkedReaderState::EndOfLine);

        self.state = ChunkedReaderState::InBody;
        let stream = self.stream.by_ref().take(self.chunk_length);
        ChunkDataReader {
            stream,
            amount_read: &mut self.chunk_amount_read,
        }
    }

    /// Finishes reading a chunk.
    ///
    /// If the chunk size was 0, the caller must call [Self::read_trailer] next.
    /// Otherwise, the caller must use [Self::read_trailer].
    ///
    /// Panics if called out of sequence.
    pub fn end_chunk(&mut self) -> Result<(), HTTPError> {
        tracing::debug!("end_chunk");
        assert!(self.state == ChunkedReaderState::InBody);

        if self.chunk_amount_read != self.chunk_length {
            return Err(HTTPError::UnexpectedEnd);
        }

        if self.chunk_length == 0 {
            self.state = ChunkedReaderState::StartOfTrailer;
        } else {
            self.read_chunk_deliminator()?;
            self.state = ChunkedReaderState::StartOfLine;
        }

        Ok(())
    }

    fn read_chunk_deliminator(&mut self) -> Result<(), HTTPError> {
        tracing::debug!("read_chunk_deliminator");

        self.buffer.clear();
        self.stream.read_limit_until(b'\n', &mut self.buffer, 2)?;
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
        assert!(self.state == ChunkedReaderState::StartOfTrailer);

        self.buffer.clear();

        crate::header::read_until_boundary(
            &mut self.stream,
            &mut self.buffer,
            self.buffer_limit,
        )?;

        let parser = HeaderParser::new();
        let header_map = parser
            .parse_header(crate::stringutil::trim_trailing_crlf(&self.buffer))
            .map_err(|error| HTTPError::InvalidTransferCoding {
                source: Some(Box::new(error)),
            })?;

        self.state = ChunkedReaderState::EndOfTrailer;

        Ok(header_map)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkedReaderState {
    StartOfLine,
    EndOfLine,
    InBody,
    StartOfTrailer,
    EndOfTrailer,
}

/// Reader for a chunk's data.
pub struct ChunkDataReader<'a, R: BufRead> {
    stream: Take<&'a mut R>,
    amount_read: &'a mut u64,
}

impl<'a, R: BufRead> Read for ChunkDataReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let amount = self.stream.read(buf)?;
        *self.amount_read += amount as u64;
        Ok(amount)
    }
}

impl <'a,R:BufRead> BufRead for ChunkDataReader<'a, R> {
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
        Err(error) => Err(HTTPError::InvalidTransferCoding {
            source: Some(Box::new(NomParseError::from_nom(line, &error))),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_reader() {
        let body = Cursor::new(b"3\r\nabc\r\n5\r\nhello\r\n0\r\nk1:v2\r\n\r\n");
        let mut reader = ChunkedEncodingReader::new(body);

        fn read_chunk<R: BufRead>(reader: &mut ChunkedEncodingReader<R>, expected: &[u8]) {
            let mut buffer = Vec::new();
            let metadata = reader.begin_chunk().unwrap();
            assert_eq!(metadata.length, expected.len() as u64);

            let mut data_reader = reader.read_data();
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
}
