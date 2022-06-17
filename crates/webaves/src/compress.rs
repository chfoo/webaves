//! Compression helpers.

use std::io::{BufReader, ErrorKind, Read, Seek};

use flate2::bufread::MultiGzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::stream::StreamOffset;

#[allow(clippy::large_enum_variant)]
enum Decoder<'a, S: Read + Seek> {
    Raw(S),
    Gzip(MultiGzDecoder<BufReader<S>>),
    Zstd(ZstdDecoder<'a, BufReader<S>>),
}

/// Decompression of Gzip and Zstd files.
pub struct Decompressor<'a, S: Read + Seek> {
    decoder: Decoder<'a, S>,
}

impl<'a, S: Read + Seek> Decompressor<'a, S> {
    fn new_impl(mut stream: S, allow_unknown: bool) -> std::io::Result<Self> {
        let mut magic_bytes = [0u8; 4];
        stream.read_exact(&mut magic_bytes)?;
        stream.seek(std::io::SeekFrom::Current(-4))?;

        let decoder = match magic_bytes {
            [0x1f, 0x8b, _, _] => Decoder::Gzip(MultiGzDecoder::new(BufReader::new(stream))),
            [0x28, 0xb5, 0x2f, 0xfd] | [0x37, 0xa4, 0x30, 0xec] => {
                Decoder::Zstd(ZstdDecoder::new(stream)?)
            }
            _ => {
                if allow_unknown {
                    Decoder::Raw(stream)
                } else {
                    return Err(ErrorKind::InvalidData.into());
                }
            }
        };

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
}

impl<'a, S: Read + Seek> Read for Decompressor<'a, S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.decoder {
            Decoder::Raw(stream) => stream.read(buf),
            Decoder::Gzip(stream) => stream.read(buf),
            Decoder::Zstd(stream) => stream.read(buf),
        }
    }
}

impl<'a, S: Read + Seek> StreamOffset for Decompressor<'a, S> {
    fn stream_offset(&mut self) -> std::io::Result<u64> {
        match &mut self.decoder {
            Decoder::Raw(stream) => stream.stream_position(),
            Decoder::Gzip(stream) => stream.get_mut().stream_position(),
            Decoder::Zstd(stream) => stream.get_mut().stream_position(),
        }
    }
}
