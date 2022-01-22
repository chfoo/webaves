use async_trait::async_trait;

use crate::io::{AsyncRead, AsyncWrite};

pub struct CaptureSink<R: AsyncWrite, W: AsyncWrite> {
    pub read_sink: R,
    pub write_sink: W,
}

impl<R: AsyncWrite, W: AsyncWrite> CaptureSink<R, W> {
    pub fn new(read_sink: R, write_sink: W) -> Self {
        Self {
            read_sink,
            write_sink,
        }
    }

    pub fn into_inner(self) -> (R, W) {
        (self.read_sink, self.write_sink)
    }
}

pub struct SourceCapture<S, CR, CW>
where
    S: AsyncRead + AsyncWrite,
    CR: AsyncWrite,
    CW: AsyncWrite,
{
    source: S,
    capture: CaptureSink<CR, CW>,
}

impl<S, CR, CW> SourceCapture<S, CR, CW>
where
    S: AsyncRead + AsyncWrite,
    CR: AsyncWrite,
    CW: AsyncWrite,
{
    pub fn new(source: S, capture: CaptureSink<CR, CW>) -> Self {
        Self { source, capture }
    }

    pub fn inner(&self) -> (&S, &CaptureSink<CR, CW>) {
        (&self.source, &self.capture)
    }

    pub fn into_inner(self) -> (S, CaptureSink<CR, CW>) {
        (self.source, self.capture)
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    pub fn capture(&self) -> &CaptureSink<CR, CW> {
        &self.capture
    }

    pub fn capture_mut(&mut self) -> &mut CaptureSink<CR, CW> {
        &mut self.capture
    }
}

#[async_trait(?Send)]
impl<S, CR, CW> AsyncRead for SourceCapture<S, CR, CW>
where
    S: AsyncRead + AsyncWrite,
    CR: AsyncWrite,
    CW: AsyncWrite,
{
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.source.read(buf).await?;
        self.capture.read_sink.write_all(&buf[0..size]).await?;
        Ok(size)
    }
}

#[async_trait(?Send)]
impl<S, CR, CW> AsyncWrite for SourceCapture<S, CR, CW>
where
    S: AsyncRead + AsyncWrite,
    CR: AsyncWrite,
    CW: AsyncWrite,
{
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let size = self.source.write(buf).await?;
        self.capture.write_sink.write_all(&buf[0..size]).await?;
        Ok(size)
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        self.source.flush().await?;
        self.capture.read_sink.flush().await?;
        self.capture.write_sink.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::io::{AsyncStreamAdapter, AsyncWriteAdapter};

    use super::*;

    #[tokio::test]
    async fn test_source_capture_pair() {
        let capture_sink = CaptureSink::new(
            AsyncWriteAdapter::new(Vec::new()),
            AsyncWriteAdapter::new(Vec::new()),
        );
        let source = AsyncStreamAdapter::new(Cursor::new(Vec::new()));
        let mut stream = SourceCapture::new(source, capture_sink);

        assert_eq!(stream.write(b"hello ").await.unwrap(), 6);
        stream.write_all(b"world!").await.unwrap();

        stream.flush().await.unwrap();
        stream.source_mut().inner_mut().set_position(0);
        stream.source_mut().inner_mut().get_mut().clear();
        stream
            .source_mut()
            .inner_mut()
            .get_mut()
            .extend_from_slice(b"12345678");

        let mut buffer = [0u8; 2];
        assert_eq!(stream.read(&mut buffer).await.unwrap(), 2);
        assert_eq!(buffer[0], b'1');
        assert_eq!(buffer[1], b'2');
        stream.read_exact(&mut buffer).await.unwrap();
        assert_eq!(buffer[0], b'3');
        assert_eq!(buffer[1], b'4');

        let (_source, capture_sink) = stream.into_inner();
        let (read_sink, write_sink) = capture_sink.into_inner();

        assert_eq!(write_sink.inner(), b"hello world!");
        assert_eq!(read_sink.inner(), b"1234");
    }
}
