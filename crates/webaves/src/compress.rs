//! Compression and decompression streams.

use std::io::{ErrorKind, Read, Write};

use flate2::Compression as GzCompression;
use flate2::{bufread::MultiGzDecoder, write::GzEncoder};
use zstd::stream::read::Decoder as ZstdDecoder;
use zstd::stream::write::Encoder as ZstdEncoder;

use crate::stream::{CountBufReader, CountRead, PeekReader};

/// Specifies a compression or decompression format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    /// Apply no codec. Pass data through as is.
    Raw,
    /// Gzip file format.
    Gzip,
    /// Zstandard file format.
    Zstd,
}

#[allow(clippy::large_enum_variant)]
enum Decoder<'a, S: Read> {
    Raw(CountBufReader<PeekReader<S>>),
    Gzip(MultiGzDecoder<CountBufReader<PeekReader<S>>>),
    Zstd(ZstdDecoder<'a, CountBufReader<PeekReader<S>>>),
}

impl<'a, S: Read> Decoder<'a, S> {
    fn name(&self) -> &'static str {
        match self {
            Decoder::Raw(_) => "raw",
            Decoder::Gzip(_) => "gzip",
            Decoder::Zstd(_) => "zstd",
        }
    }
}

/// Decompression of Gzip and Zstd files.
pub struct Decompressor<'a, S: Read> {
    decoder: Decoder<'a, S>,
}

impl<'a, S: Read> Decompressor<'a, S> {
    fn new_impl(stream: S, allow_unknown: bool) -> std::io::Result<Self> {
        let mut stream = PeekReader::new(stream);
        let magic_bytes = stream.peek(4)?.to_vec();
        let stream = CountBufReader::new(stream);

        tracing::debug!(?magic_bytes, "decompressor analysis");

        let decoder = match &magic_bytes[0..4] {
            [0x1f, 0x8b, _, _] => Decoder::Gzip(MultiGzDecoder::new(stream)),
            [0x28, 0xb5, 0x2f, 0xfd] | [0x37, 0xa4, 0x30, 0xec] => {
                Decoder::Zstd(ZstdDecoder::with_buffer(stream)?)
            }
            _ => {
                if allow_unknown {
                    Decoder::Raw(stream)
                } else {
                    return Err(ErrorKind::InvalidData.into());
                }
            }
        };
        tracing::debug!(decoder = decoder.name(), "decoder select");

        Ok(Self { decoder })
    }

    /// Open a compressed file.
    ///
    /// Returns error for unsupported compression formats.
    pub fn new(stream: S) -> std::io::Result<Self> {
        Self::new_impl(stream, false)
    }

    /// Open a compressed file or contents unchanged for unsupported formats.
    pub fn new_allow_unknown(stream: S) -> std::io::Result<Self> {
        Self::new_impl(stream, true)
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &S {
        match &self.decoder {
            Decoder::Raw(stream) => stream.get_ref().get_ref(),
            Decoder::Gzip(stream) => stream.get_ref().get_ref().get_ref(),
            Decoder::Zstd(stream) => stream.get_ref().get_ref().get_ref(),
        }
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut S {
        match &mut self.decoder {
            Decoder::Raw(stream) => stream.get_mut().get_mut(),
            Decoder::Gzip(stream) => stream.get_mut().get_mut().get_mut(),
            Decoder::Zstd(stream) => stream.get_mut().get_mut().get_mut(),
        }
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> S {
        match self.decoder {
            Decoder::Raw(stream) => stream.into_inner().into_inner(),
            Decoder::Gzip(stream) => stream.into_inner().into_inner().into_inner(),
            Decoder::Zstd(stream) => stream.finish().into_inner().into_inner(),
        }
    }

    /// Returns the number of bytes read from the wrapped stream.
    pub fn raw_input_read_count(&self) -> u64 {
        match &self.decoder {
            Decoder::Raw(stream) => stream.read_count(),
            Decoder::Gzip(stream) => stream.get_ref().read_count(),
            Decoder::Zstd(stream) => stream.get_ref().read_count(),
        }
    }
}

impl<'a, S: Read> Read for Decompressor<'a, S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.decoder {
            Decoder::Raw(stream) => stream.read(buf),
            Decoder::Gzip(stream) => stream.read(buf),
            Decoder::Zstd(stream) => stream.read(buf),
        }
    }
}

enum Encoder<'a, S: Write> {
    Raw(S),
    Gzip(GzEncoder<S>),
    Zstd(ZstdEncoder<'a, S>),
}

/// Specifies a compression level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    /// Fastest speed but with low compression ratio.
    Fast,

    /// Default level specified by the codec.
    CodecDefault,

    /// Recommended balanced ratio of speed and compression.
    ///
    /// Default value.
    Optimal,

    /// Almost best compression ratio at the cost of slow speed.
    High,
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::Optimal
    }
}

impl CompressionLevel {
    fn get_int_for_format(&self, format: CompressionFormat) -> i32 {
        match format {
            CompressionFormat::Raw => 0,
            CompressionFormat::Gzip => match self {
                CompressionLevel::Fast => 1,
                CompressionLevel::CodecDefault => 6,
                CompressionLevel::Optimal => 9,
                CompressionLevel::High => 9,
            },
            CompressionFormat::Zstd => match self {
                CompressionLevel::Fast => 1,
                CompressionLevel::CodecDefault => 3,
                CompressionLevel::Optimal => 3,
                CompressionLevel::High => 19,
            },
        }
    }
}

/// Compression of Gzip and Zstd files.
pub struct Compressor<'a, S: Write> {
    encoder: Encoder<'a, S>,
}

impl<'a, S: Write> Compressor<'a, S> {
    /// Create a compressor with the given stream and codec options.
    pub fn new(stream: S, format: CompressionFormat, level: CompressionLevel) -> std::io::Result<Self> {
        let encoder = match format {
            CompressionFormat::Raw => Encoder::Raw(stream),
            CompressionFormat::Gzip => Encoder::Gzip(GzEncoder::new(
                stream,
                GzCompression::new(level.get_int_for_format(format) as u32),
            )),
            CompressionFormat::Zstd => {
                Encoder::Zstd(ZstdEncoder::new(stream, level.get_int_for_format(format))?)
            }
        };
        Ok(Self { encoder })
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &S {
        match &self.encoder {
            Encoder::Raw(stream) => stream,
            Encoder::Gzip(stream) => stream.get_ref(),
            Encoder::Zstd(stream) => stream.get_ref(),
        }
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut S {
        match &mut self.encoder {
            Encoder::Raw(stream) => stream,
            Encoder::Gzip(stream) => stream.get_mut(),
            Encoder::Zstd(stream) => stream.get_mut(),
        }
    }

    /// Completes a compression file and returns the wrapped stream.
    pub fn finish(self) -> std::io::Result<S> {
        match self.encoder {
            Encoder::Raw(stream) => Ok(stream),
            Encoder::Gzip(stream) => stream.finish(),
            Encoder::Zstd(stream) => stream.finish()
        }
    }
}

impl<'a, S: Write> Write for Compressor<'a, S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match &mut self.encoder {
            Encoder::Raw(stream) => stream.write(buf),
            Encoder::Gzip(stream) => stream.write(buf),
            Encoder::Zstd(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.encoder {
            Encoder::Raw(stream) => stream.flush(),
            Encoder::Gzip(stream) => stream.flush(),
            Encoder::Zstd(stream) => stream.flush(),
        }
    }
}
