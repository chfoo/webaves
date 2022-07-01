use std::io::Write;

use super::{HTTPError, RequestHeader, ResponseHeader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriterState {
    Header,
    Body,
}

/// HTTP request and response writer.
pub struct MessageWriter<W: Write> {
    stream: Option<W>,
    body_writer: Option<BodyWriter<W>>,
    state: WriterState,
}

impl<W: Write> MessageWriter<W> {
    /// Creates a new `MessageWriter` with the given stream.
    pub fn new(stream: W) -> Self {
        Self {
            stream: Some(stream),
            body_writer: None,
            state: WriterState::Header,
        }
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &W {
        match self.stream.as_ref() {
            Some(stream) => stream,
            None => self.body_writer.as_ref().unwrap().get_ref(),
        }
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut W {
        match self.stream.as_mut() {
            Some(stream) => stream,
            None => self.body_writer.as_mut().unwrap().get_mut(),
        }
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> W {
        match self.stream {
            Some(stream) => stream,
            None => self.body_writer.unwrap().into_inner(),
        }
    }

    /// Begins writing a HTTP request.
    ///
    /// [Self::write_body] or [Self::end_message] must be called next.
    ///
    /// Panics when called out of sequence.
    pub fn begin_request(&mut self, header: &RequestHeader) -> Result<(), HTTPError> {
        tracing::debug!("begin_request");
        assert!(self.state == WriterState::Header);

        let mut stream = self.stream.as_mut().unwrap();

        header.format(&mut stream)?;
        stream.write_all(b"\r\n")?;
        stream.flush()?;
        self.set_up_body_writer();
        self.state = WriterState::Body;

        Ok(())
    }

    /// Begins writing a HTTP response.
    ///
    /// [Self::write_body] or [Self::end_message] must be called next.
    ///
    /// Panics when called out of sequence.
    pub fn begin_response(&mut self, header: &ResponseHeader) -> Result<(), HTTPError> {
        tracing::debug!("begin_response");
        assert!(self.state == WriterState::Header);

        let mut stream = self.stream.as_mut().unwrap();

        header.format(&mut stream)?;
        stream.write_all(b"\r\n")?;
        stream.flush()?;
        self.set_up_body_writer();
        self.state = WriterState::Body;

        Ok(())
    }

    fn set_up_body_writer(&mut self) {
        let stream = self.stream.take().unwrap();

        self.body_writer = Some(BodyWriter { stream });
    }

    /// Returns a writer for writing the message body.
    ///
    /// Once the message body has been write, [Self::end_message] must be
    /// called next.
    ///
    /// Panics when called out of sequence.
    pub fn write_body(&mut self) -> &mut BodyWriter<W> {
        assert!(self.state == WriterState::Body);

        self.body_writer.as_mut().unwrap()
    }

    /// Finishes writing the message.
    ///
    /// [Self::begin_request] or [Self::begin_response] may be called next if
    /// the protocol allows it.
    ///
    /// Panics when called out of sequence.
    pub fn end_message(&mut self) -> Result<(), HTTPError> {
        tracing::debug!("end_message");
        assert!(self.state == WriterState::Body);

        let mut stream = self.body_writer.take().unwrap().into_inner();
        stream.flush()?;
        self.stream = Some(stream);

        self.state = WriterState::Header;

        Ok(())
    }
}

/// Writer for a message body.
pub struct BodyWriter<W: Write> {
    stream: W,
}

impl<W: Write> BodyWriter<W> {
    fn get_ref(&self) -> &W {
        &self.stream
    }

    fn get_mut(&mut self) -> &mut W {
        &mut self.stream
    }

    fn into_inner(self) -> W {
        self.stream
    }
}

impl<W: Write> Write for BodyWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}
