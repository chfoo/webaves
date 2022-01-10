use async_trait::async_trait;
use bytes::{Buf, BufMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BUFFER_SIZE: usize = 4096;

#[async_trait(?Send)]
pub trait AsyncRead {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;

    async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        let mut offset = 0;

        while offset < buf.len() {
            let size_read = self.read(&mut buf[offset..]).await?;

            if size_read == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
            }

            offset += size_read;
        }

        Ok(())
    }

    async fn read_buf<B: BufMut>(&mut self, buf: &mut B) -> std::io::Result<usize> {
        let mut temp_buffer = [0u8; BUFFER_SIZE];

        let size = self.read(&mut temp_buffer).await?;
        buf.put_slice(&temp_buffer);

        Ok(size)
    }

    async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let mut total_size = 0;

        loop {
            let size = self.read_buf(buf).await?;

            if size == 0 {
                break;
            }

            total_size += size;
        }

        Ok(total_size)
    }
}

#[async_trait(?Send)]
pub trait AsyncWrite {
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;

    async fn flush(&mut self) -> std::io::Result<()>;

    async fn write_buf<B: Buf>(&mut self, buf: &mut B) -> std::io::Result<usize> {
        let size = self.write(buf.chunk()).await?;
        buf.advance(size);

        Ok(size)
    }

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let mut offset = 0;

        while offset < buf.len() {
            let size_written = self.write(&buf[offset..]).await?;

            if size_written == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::WriteZero));
            }

            offset += size_written;
        }

        Ok(())
    }

    async fn write_all_buf<B: Buf>(&mut self, buf: &mut B) -> std::io::Result<()> {
        while buf.has_remaining() {
            let size = self.write(buf.chunk()).await?;
            buf.advance(size);
        }

        Ok(())
    }
}

pub struct AsyncReadAdapter<R: tokio::io::AsyncRead> {
    inner: R,
}

impl<R: tokio::io::AsyncRead> From<R> for AsyncReadAdapter<R> {
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl<R: tokio::io::AsyncRead> AsyncReadAdapter<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &R {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

#[async_trait(?Send)]
impl<W: tokio::io::AsyncRead + Unpin> AsyncRead for AsyncReadAdapter<W> {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(self.inner.read(buf).await?)
    }
}

pub struct AsyncWriteAdapter<W: tokio::io::AsyncWrite> {
    inner: W,
}

impl<W: tokio::io::AsyncWrite> From<W> for AsyncWriteAdapter<W> {
    fn from(value: W) -> Self {
        Self::new(value)
    }
}

impl<W: tokio::io::AsyncWrite> AsyncWriteAdapter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &W {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[async_trait(?Send)]
impl<W: tokio::io::AsyncWrite + Unpin> AsyncWrite for AsyncWriteAdapter<W> {
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(self.inner.write(buf).await?)
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        Ok(self.inner.flush().await?)
    }
}

pub struct AsyncStreamAdapter<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    inner: S,
}

impl<S> From<S> for AsyncStreamAdapter<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    fn from(value: S) -> Self {
        Self::new(value)
    }
}

impl<S> AsyncStreamAdapter<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

#[async_trait(?Send)]
impl<S> AsyncRead for AsyncStreamAdapter<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(self.inner.read(buf).await?)
    }
}

#[async_trait(?Send)]
impl<S> AsyncWrite for AsyncStreamAdapter<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(self.inner.write(buf).await?)
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        Ok(self.inner.flush().await?)
    }
}
