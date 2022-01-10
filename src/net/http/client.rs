use bytes::{Buf, BufMut, BytesMut};
use headers::{HeaderMapExt, TransferEncoding};
use http::{HeaderMap, Request, Response, Version};
use tracing::{debug, trace};

use crate::io::{AsyncRead, AsyncWrite};

use super::{
    util::{self, RequestTarget},
    HttpError,
};

const BUFFER_SIZE: usize = 16384;

#[derive(Debug)]
enum ResponseBodyState {
    Plain,
    StartChunk,
    InChunk { bytes_read: u64, chunk_size: u64 },
}

pub struct WireProtocol<S>
where
    S: AsyncRead + AsyncWrite,
{
    stream: S,
    buffer: BytesMut,
    response_body_state: ResponseBodyState,
}

impl<S> WireProtocol<S>
where
    S: AsyncRead + AsyncWrite,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: BytesMut::new(),
            response_body_state: ResponseBodyState::Plain,
        }
    }

    pub fn inner(&self) -> &S {
        &self.stream
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    pub fn into_inner(self) -> S {
        self.stream
    }

    pub async fn write_request(
        &mut self,
        request: Request<()>,
        target: RequestTarget,
    ) -> Result<(), HttpError> {
        let request_line = util::format_request_line(&request, target);

        debug!(?request_line);

        self.stream.write_all(request_line.as_bytes()).await?;

        self.buffer.clear();
        util::serialize_headers(request.headers(), &mut self.buffer);

        self.stream.write_all(&self.buffer).await?;
        self.stream.write_all(util::NEWLINE).await?;
        self.stream.flush().await?;

        Ok(())
    }

    pub async fn write_body(&mut self, data: &[u8]) -> Result<(), HttpError> {
        self.stream.write_all(data).await?;
        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<Response<()>, HttpError> {
        self.response_body_state = ResponseBodyState::Plain;
        self.buffer.clear();
        self.fill_read_buffer().await?;

        if !self.buffer.starts_with(b"HTTP/") {
            debug!("HTTP/0.9 response");

            return Ok(Response::builder().version(Version::HTTP_09).body(())?);
        }

        let mut parser_headers = [httparse::EMPTY_HEADER; 128];
        let mut parser_response = httparse::Response::new(&mut parser_headers);

        match parser_response.parse(self.buffer.as_ref())? {
            httparse::Status::Complete(size) => {
                let response = util::convert_parser_response(&parser_response)?;
                self.buffer.advance(size);

                if let Some(t) = response.headers().typed_get::<TransferEncoding>() {
                    if t.is_chunked() {
                        self.response_body_state = ResponseBodyState::StartChunk;
                    }
                };

                debug!(?self.response_body_state, "read response");

                Ok(response)
            }
            httparse::Status::Partial => Err(HttpError::Incomplete("partial response header")),
        }
    }

    pub async fn read_body(&mut self, dest: &mut [u8]) -> Result<usize, HttpError> {
        if let ResponseBodyState::Plain = self.response_body_state {
            Ok(self.read_body_plain(dest).await?)
        } else {
            Ok(self.read_body_chunks(dest).await?)
        }
    }

    pub async fn read_body_buf<B: BufMut>(&mut self, dest: &mut B) -> Result<usize, HttpError> {
        let mut local_buffer = [0u8; BUFFER_SIZE];

        let size = self.read_body(&mut local_buffer).await?;
        dest.put_slice(&local_buffer[0..size]);

        Ok(size)
    }

    async fn read_body_plain(&mut self, dest: &mut [u8]) -> Result<usize, HttpError> {
        debug_assert!(matches!(self.response_body_state, ResponseBodyState::Plain));

        if self.buffer.is_empty() {
            Ok(self.stream.read(dest).await?)
        } else {
            let size = dest.len().min(self.buffer.len());
            let data = self.buffer.split_to(size);
            dest[0..size].copy_from_slice(&data);
            Ok(size)
        }
    }

    async fn read_body_chunks(&mut self, dest: &mut [u8]) -> Result<usize, HttpError> {
        if let ResponseBodyState::StartChunk = self.response_body_state {
            self.read_chunk_size().await?;
        }

        if let ResponseBodyState::InChunk {
            bytes_read,
            chunk_size,
        } = self.response_body_state
        {
            return self.read_chunk_content(dest, bytes_read, chunk_size).await;
        }

        unreachable!();
    }

    async fn read_chunk_size(&mut self) -> Result<(), HttpError> {
        self.fill_read_buffer().await?;

        match httparse::parse_chunk_size(&self.buffer)? {
            httparse::Status::Complete((chunk_meta_size, chunk_size)) => {
                self.buffer.advance(chunk_meta_size);

                self.response_body_state = ResponseBodyState::InChunk {
                    bytes_read: 0,
                    chunk_size,
                };
                trace!(?self.response_body_state, "read chunk size");

                Ok(())
            }
            httparse::Status::Partial => Err(HttpError::Incomplete("partial chunk size")),
        }
    }

    async fn read_chunk_content(
        &mut self,
        dest: &mut [u8],
        bytes_read: u64,
        chunk_size: u64,
    ) -> Result<usize, HttpError> {
        self.fill_read_buffer().await?;

        let size = self.buffer.len().min(chunk_size as usize).min(dest.len());

        if size == 0 {
            return Ok(0);
        }

        let data = self.buffer.split_to(size);
        dest[0..size].copy_from_slice(&data.as_ref()[0..size]);

        let new_bytes_read = bytes_read + size as u64;

        if new_bytes_read == chunk_size {
            self.consume_buffer_newline();
            self.response_body_state = ResponseBodyState::StartChunk;
        } else {
            self.response_body_state = ResponseBodyState::InChunk {
                bytes_read: new_bytes_read,
                chunk_size,
            };
        }

        trace!(size, new_bytes_read, ?self.response_body_state, "read chunk content");

        Ok(size)
    }

    fn consume_buffer_newline(&mut self) {
        if self.buffer.starts_with(b"\r\n") {
            self.buffer.advance(2);
        } else if self.buffer.starts_with(b"\n") {
            self.buffer.advance(1);
        }
    }

    pub async fn read_trailer(&mut self) -> Result<HeaderMap, HttpError> {
        if let ResponseBodyState::Plain = self.response_body_state {
            return Ok(HeaderMap::new());
        }

        self.fill_read_buffer().await?;

        let mut parser_headers = [httparse::EMPTY_HEADER; 128];

        match httparse::parse_headers(&self.buffer, &mut parser_headers)? {
            httparse::Status::Complete((_size, parsed_headers)) => {
                let mut headers = HeaderMap::new();
                util::convert_parser_headers(parsed_headers, &mut headers)?;

                debug!(trailer_count = headers.len());

                Ok(headers)
            }
            httparse::Status::Partial => Err(HttpError::Incomplete("partial trailers")),
        }
    }

    async fn fill_read_buffer(&mut self) -> std::io::Result<()> {
        let mut temp_buffer = [0u8; BUFFER_SIZE];

        while self.buffer.len() < BUFFER_SIZE {
            let size = self.stream.read(&mut temp_buffer).await?;

            if size == 0 {
                break;
            }

            self.buffer.put_slice(&temp_buffer[0..size]);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use headers::HeaderValue;

    use crate::io::AsyncStreamAdapter;

    use super::*;

    fn make_wire(data: &[u8]) -> WireProtocol<AsyncStreamAdapter<Cursor<Vec<u8>>>> {
        WireProtocol::new(AsyncStreamAdapter::new(Cursor::new(Vec::from(data))))
    }

    #[test_log::test(tokio::test)]
    async fn test_wire_protocol_write_header() {
        let mut wire = make_wire(b"");

        wire.write_request(
            Request::builder()
                .uri("/index.html")
                .header("Host", "example.com")
                .body(())
                .unwrap(),
            RequestTarget::Origin,
        )
        .await
        .unwrap();
        wire.write_body(b"hello world").await.unwrap();

        let data = wire.into_inner().into_inner().into_inner();
        assert_eq!(
            &data,
            b"GET /index.html HTTP/1.1\r\n\
            host: example.com\r\n\
            \r\n\
            hello world"
        );
    }

    #[test_log::test(tokio::test)]
    async fn test_wire_protocol_plain() {
        let mut wire = make_wire(
            b"HTTP/1.1 307 Temporary redirect\r\n\
            Content-type: text/plain\r\n\
            \r\n\
            Hello world!",
        );

        let response = wire.read_response().await.unwrap();

        assert_eq!(response.status(), 307);
        assert_eq!(
            response.headers().get("Content-Type"),
            Some(&HeaderValue::from_bytes(b"text/plain").unwrap())
        );

        let mut body = Vec::new();

        loop {
            let size = wire.read_body_buf(&mut body).await.unwrap();

            if size == 0 {
                break;
            }
        }

        assert_eq!(&body, b"Hello world!");
    }

    #[test_log::test(tokio::test)]
    async fn test_wire_protocol_chunked() {
        let mut wire = make_wire(
            b"HTTP/1.1 307 Temporary redirect\r\n\
            Content-type: text/plain\r\n\
            Transfer-encoding: chunked\r\n\
            \r\n\
            6\r\n\
            Hello \r\n\
            8\r\n\
            world!!!\r\n\
            0; abc\r\n\
            N1: V1\r\n\
            \r\n",
        );

        let response = wire.read_response().await.unwrap();

        assert_eq!(response.status(), 307);

        let mut body = Vec::new();

        loop {
            let size = wire.read_body_buf(&mut body).await.unwrap();

            if size == 0 {
                break;
            }
        }

        assert_eq!(&body, b"Hello world!!!");

        let trailer = wire.read_trailer().await.unwrap();
        assert_eq!(
            trailer.get("n1"),
            Some(&HeaderValue::from_bytes(b"V1").unwrap())
        );
    }
}
