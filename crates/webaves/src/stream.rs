//! Stream helpers.

/// Provides position information in the stream.
pub trait StreamOffset {
    /// Returns the current position from the start of the stream.
    fn stream_offset(&mut self) -> std::io::Result<u64>;
}
