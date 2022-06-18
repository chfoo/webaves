//! Compression helpers.

use std::io::{BufReader, ErrorKind, Read};

use flate2::bufread::MultiGzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::stream::{CountBufReader, CountRead, PeekReader};

#[allow(clippy::large_enum_variant)]
enum Decoder<'a, S: Read> {
    Raw(CountBufReader<BufReader<PeekReader<S>>>),
    Gzip(MultiGzDecoder<CountBufReader<BufReader<PeekReader<S>>>>),
    Zstd(ZstdDecoder<'a, CountBufReader<BufReader<PeekReader<S>>>>),
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
        let stream = CountBufReader::new(BufReader::new(stream));

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

    pub fn get_ref(&self) -> &S {
        match &self.decoder {
            Decoder::Raw(stream) => stream.get_ref().get_ref().get_ref(),
            Decoder::Gzip(stream) => stream.get_ref().get_ref().get_ref().get_ref(),
            Decoder::Zstd(stream) => stream.get_ref().get_ref().get_ref().get_ref(),
        }
    }

    pub fn get_mut(&mut self) -> &mut S {
        match &mut self.decoder {
            Decoder::Raw(stream) => stream.get_mut().get_mut().get_mut(),
            Decoder::Gzip(stream) => stream.get_mut().get_mut().get_mut().get_mut(),
            Decoder::Zstd(stream) => stream.get_mut().get_mut().get_mut().get_mut(),
        }
    }

    pub fn into_inner(self) -> S {
        match self.decoder {
            Decoder::Raw(stream) => stream.into_inner().into_inner().into_inner(),
            Decoder::Gzip(stream) => stream.into_inner().into_inner().into_inner().into_inner(),
            Decoder::Zstd(stream) => stream.finish().into_inner().into_inner().into_inner(),
        }
    }

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
