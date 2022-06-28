//! Compression and decompression streams.

use std::{
    io::{ErrorKind, Read, Write},
    str::FromStr,
};

use brotli::enc::writer::CompressorWriter as BrotliEncoder;
use brotli::Decompressor as BrotliDecoder;
use flate2::{bufread::MultiGzDecoder, write::GzEncoder};
use flate2::{
    bufread::{DeflateDecoder, ZlibDecoder},
    write::{DeflateEncoder, ZlibEncoder},
    Compression as GzCompression,
};
use zstd::stream::read::Decoder as ZstdDecoder;
use zstd::stream::write::Encoder as ZstdEncoder;

use crate::stream::{CountBufReader, CountRead, PeekReader};

/// Specifies a compression or decompression format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    /// Apply no codec. Pass data through as is.
    Raw,
    /// DEFLATE raw stream.
    DeflateRaw,
    /// DEFLATE in Zlib wrapper file format.
    DeflateZlib,
    /// Gzip file format.
    Gzip,
    /// Brotli raw stream.
    Brotli,
    /// Zstandard file format.
    Zstd,
}

impl CompressionFormat {
    /// Returns the HTTP content coding name
    pub fn as_coding_name_str(&self) -> &'static str {
        match self {
            CompressionFormat::Raw => "identity",
            CompressionFormat::DeflateRaw => "deflate",
            CompressionFormat::DeflateZlib => "deflate",
            CompressionFormat::Gzip => "gzip",
            CompressionFormat::Brotli => "br",
            CompressionFormat::Zstd => "zstd",
        }
    }
}

impl FromStr for CompressionFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "identity" => Ok(Self::Raw),
            "deflate" => Ok(Self::DeflateZlib),
            "gzip" | "x-gzip" | "gz" => Ok(Self::Gzip),
            "brotli" | "br" => Ok(Self::Brotli),
            "zstd" => Ok(Self::Zstd),
            _ => Err(()),
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum Decoder<'a, S: Read> {
    Raw(CountBufReader<PeekReader<S>>),
    DeflateRaw(DeflateDecoder<CountBufReader<PeekReader<S>>>),
    DeflateZlib(ZlibDecoder<CountBufReader<PeekReader<S>>>),
    Gzip(MultiGzDecoder<CountBufReader<PeekReader<S>>>),
    Brotli(BrotliDecoder<CountBufReader<PeekReader<S>>>),
    Zstd(ZstdDecoder<'a, CountBufReader<PeekReader<S>>>),
}

impl<'a, S: Read> Decoder<'a, S> {
    fn name(&self) -> &'static str {
        match self {
            Decoder::Raw(_) => "raw",
            Decoder::DeflateRaw(_) => "deflate-raw",
            Decoder::DeflateZlib(_) => "deflate-zlib",
            Decoder::Gzip(_) => "gzip",
            Decoder::Brotli(_) => "brotli",
            Decoder::Zstd(_) => "zstd",
        }
    }
}

/// Decompression of Zlib/Deflate, Gzip, Brotli, and Zstd files.
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
            [0x78, 0x01, _, _] | [0x78, 0x5e, _, _] | [0x78, 0x9c, _, _] | [0x78, 0xda, _, _] => {
                Decoder::DeflateZlib(ZlibDecoder::new(stream))
            }
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
    /// Returns error for unsupported or undetectable compression formats.
    pub fn new(stream: S) -> std::io::Result<Self> {
        Self::new_impl(stream, false)
    }

    /// Open a compressed file or contents unchanged for unsupported or undetectable formats.
    pub fn new_allow_unknown(stream: S) -> std::io::Result<Self> {
        Self::new_impl(stream, true)
    }

    /// Open a compressed file with the given format.
    pub fn new_format(stream: S, format: CompressionFormat) -> std::io::Result<Self> {
        let stream = PeekReader::new(stream);
        let stream = CountBufReader::new(stream);
        let decoder = match format {
            CompressionFormat::Raw => Decoder::Raw(stream),
            CompressionFormat::DeflateRaw => Decoder::DeflateRaw(DeflateDecoder::new(stream)),
            CompressionFormat::DeflateZlib => Decoder::DeflateZlib(ZlibDecoder::new(stream)),
            CompressionFormat::Gzip => Decoder::Gzip(MultiGzDecoder::new(stream)),
            CompressionFormat::Brotli => Decoder::Brotli(BrotliDecoder::new(stream, 4096)),
            CompressionFormat::Zstd => Decoder::Zstd(ZstdDecoder::with_buffer(stream)?),
        };

        Ok(Self { decoder })
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &S {
        match &self.decoder {
            Decoder::Raw(stream) => stream.get_ref().get_ref(),
            Decoder::DeflateRaw(stream) => stream.get_ref().get_ref().get_ref(),
            Decoder::DeflateZlib(stream) => stream.get_ref().get_ref().get_ref(),
            Decoder::Gzip(stream) => stream.get_ref().get_ref().get_ref(),
            Decoder::Brotli(stream) => stream.get_ref().get_ref().get_ref(),
            Decoder::Zstd(stream) => stream.get_ref().get_ref().get_ref(),
        }
    }

    /// Returns a mutable reference to the wrapped stream.
    ///
    /// Panics on Brotli.
    pub fn get_mut(&mut self) -> &mut S {
        match &mut self.decoder {
            Decoder::Raw(stream) => stream.get_mut().get_mut(),
            Decoder::DeflateRaw(stream) => stream.get_mut().get_mut().get_mut(),
            Decoder::DeflateZlib(stream) => stream.get_mut().get_mut().get_mut(),
            Decoder::Gzip(stream) => stream.get_mut().get_mut().get_mut(),
            Decoder::Brotli(_stream) => unimplemented!(),
            Decoder::Zstd(stream) => stream.get_mut().get_mut().get_mut(),
        }
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> S {
        match self.decoder {
            Decoder::Raw(stream) => stream.into_inner().into_inner(),
            Decoder::DeflateRaw(stream) => stream.into_inner().into_inner().into_inner(),
            Decoder::DeflateZlib(stream) => stream.into_inner().into_inner().into_inner(),
            Decoder::Gzip(stream) => stream.into_inner().into_inner().into_inner(),
            Decoder::Brotli(stream) => stream.into_inner().into_inner().into_inner(),
            Decoder::Zstd(stream) => stream.finish().into_inner().into_inner(),
        }
    }

    /// Returns the number of bytes read from the wrapped stream.
    pub fn raw_input_read_count(&self) -> u64 {
        match &self.decoder {
            Decoder::Raw(stream) => stream.read_count(),
            Decoder::DeflateRaw(stream) => stream.get_ref().read_count(),
            Decoder::DeflateZlib(stream) => stream.get_ref().read_count(),
            Decoder::Gzip(stream) => stream.get_ref().read_count(),
            Decoder::Brotli(stream) => stream.get_ref().read_count(),
            Decoder::Zstd(stream) => stream.get_ref().read_count(),
        }
    }
}

impl<'a, S: Read> Read for Decompressor<'a, S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.decoder {
            Decoder::Raw(stream) => stream.read(buf),
            Decoder::DeflateRaw(stream) => stream.read(buf),
            Decoder::DeflateZlib(stream) => stream.read(buf),
            Decoder::Gzip(stream) => stream.read(buf),
            Decoder::Brotli(stream) => stream.read(buf),
            Decoder::Zstd(stream) => stream.read(buf),
        }
    }
}

enum Encoder<'a, S: Write> {
    Raw(S),
    DeflateRaw(DeflateEncoder<S>),
    DeflateZlib(ZlibEncoder<S>),
    Gzip(GzEncoder<S>),
    Brotli(BrotliEncoder<S>),
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
            CompressionFormat::Gzip
            | CompressionFormat::DeflateRaw
            | CompressionFormat::DeflateZlib => match self {
                CompressionLevel::Fast => 1,
                CompressionLevel::CodecDefault => 6,
                CompressionLevel::Optimal => 9,
                CompressionLevel::High => 9,
            },
            CompressionFormat::Brotli => match self {
                CompressionLevel::Fast => 1,
                CompressionLevel::CodecDefault => 11,
                CompressionLevel::Optimal => 5,
                CompressionLevel::High => 11,
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

/// Compression of Zlib/Deflate, Gzip, Brotli, and Zstd files.
pub struct Compressor<'a, S: Write> {
    encoder: Encoder<'a, S>,
}

impl<'a, S: Write> Compressor<'a, S> {
    /// Create a compressor with the given stream and codec options.
    pub fn new(
        stream: S,
        format: CompressionFormat,
        level: CompressionLevel,
    ) -> std::io::Result<Self> {
        let encoder = match format {
            CompressionFormat::Raw => Encoder::Raw(stream),
            CompressionFormat::DeflateRaw => Encoder::DeflateRaw(DeflateEncoder::new(
                stream,
                GzCompression::new(level.get_int_for_format(format) as u32),
            )),
            CompressionFormat::DeflateZlib => Encoder::DeflateZlib(ZlibEncoder::new(
                stream,
                GzCompression::new(level.get_int_for_format(format) as u32),
            )),
            CompressionFormat::Gzip => Encoder::Gzip(GzEncoder::new(
                stream,
                GzCompression::new(level.get_int_for_format(format) as u32),
            )),
            CompressionFormat::Brotli => Encoder::Brotli(BrotliEncoder::new(
                stream,
                4096,
                level.get_int_for_format(format) as u32,
                22,
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
            Encoder::DeflateRaw(stream) => stream.get_ref(),
            Encoder::DeflateZlib(stream) => stream.get_ref(),
            Encoder::Gzip(stream) => stream.get_ref(),
            Encoder::Brotli(stream) => stream.get_ref(),
            Encoder::Zstd(stream) => stream.get_ref(),
        }
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut S {
        match &mut self.encoder {
            Encoder::Raw(stream) => stream,
            Encoder::DeflateRaw(stream) => stream.get_mut(),
            Encoder::DeflateZlib(stream) => stream.get_mut(),
            Encoder::Gzip(stream) => stream.get_mut(),
            Encoder::Brotli(stream) => stream.get_mut(),
            Encoder::Zstd(stream) => stream.get_mut(),
        }
    }

    /// Completes a compression file and returns the wrapped stream.
    pub fn finish(self) -> std::io::Result<S> {
        match self.encoder {
            Encoder::Raw(stream) => Ok(stream),
            Encoder::DeflateRaw(stream) => stream.finish(),
            Encoder::DeflateZlib(stream) => stream.finish(),
            Encoder::Gzip(stream) => stream.finish(),
            Encoder::Brotli(mut stream) => {
                stream.flush()?;
                Ok(stream.into_inner())
            }
            Encoder::Zstd(stream) => stream.finish(),
        }
    }
}

impl<'a, S: Write> Write for Compressor<'a, S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match &mut self.encoder {
            Encoder::Raw(stream) => stream.write(buf),
            Encoder::DeflateRaw(stream) => stream.write(buf),
            Encoder::DeflateZlib(stream) => stream.write(buf),
            Encoder::Gzip(stream) => stream.write(buf),
            Encoder::Brotli(stream) => stream.write(buf),
            Encoder::Zstd(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.encoder {
            Encoder::Raw(stream) => stream.flush(),
            Encoder::DeflateRaw(stream) => stream.flush(),
            Encoder::DeflateZlib(stream) => stream.flush(),
            Encoder::Gzip(stream) => stream.flush(),
            Encoder::Brotli(stream) => stream.flush(),
            Encoder::Zstd(stream) => stream.flush(),
        }
    }
}
