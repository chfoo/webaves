use bytes::BufMut;
use http::{HeaderMap, Request, Response, Version};

use super::HttpError;

pub(crate) const NEWLINE: &[u8; 2] = b"\r\n";

pub enum RequestTarget {
    Origin,
    Absolute,
    Authority,
    Asterisk,
}

pub fn format_request_line<B>(request: &Request<B>, target: RequestTarget) -> String {
    let mut _target = String::new();

    let target = match target {
        RequestTarget::Origin => request
            .uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/"),
        RequestTarget::Absolute => {
            _target = request.uri().to_string();
            &_target
        }
        RequestTarget::Authority => request.uri().authority().map(|a| a.as_str()).unwrap_or("/"),
        RequestTarget::Asterisk => "*",
    };

    format!("{} {} HTTP/1.1\r\n", request.method().as_str(), target)
}

pub fn serialize_headers<B: BufMut>(headers: &HeaderMap, dest: &mut B) {
    for (name, value) in headers {
        dest.put(name.as_str().as_bytes());
        dest.put_slice(b": ");
        dest.put(value.as_bytes());
        dest.put_slice(NEWLINE);
    }
}

pub fn convert_parser_headers(
    parser_headers: &[httparse::Header],
    dest: &mut HeaderMap,
) -> Result<(), HttpError> {
    for header in parser_headers.iter() {
        if !header.name.is_empty() {
            dest.append(
                http::header::HeaderName::from_bytes(header.name.as_bytes())?,
                header.value.try_into()?,
            );
        }
    }

    Ok(())
}

pub fn convert_parser_response(
    parser_response: &httparse::Response,
) -> Result<Response<()>, HttpError> {
    let mut response = Response::builder()
        .status(parser_response.code.unwrap_or_default())
        .version(match parser_response.version.unwrap_or_default() {
            1 => Version::HTTP_11,
            _ => Version::HTTP_10,
        })
        .body(())?;

    response.headers_mut().insert(
        "webaves-reason-phrase",
        parser_response.reason.unwrap_or_default().try_into()?,
    );

    convert_parser_headers(parser_response.headers, response.headers_mut())?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use http::HeaderValue;

    use super::*;

    #[test]
    fn test_format_request_line() {
        let request = Request::builder()
            .uri("http://example.com/index.html")
            .body(())
            .unwrap();

        let line = format_request_line(&request, RequestTarget::Origin);
        assert_eq!(&line, "GET /index.html HTTP/1.1\r\n");

        let line = format_request_line(&request, RequestTarget::Absolute);
        assert_eq!(&line, "GET http://example.com/index.html HTTP/1.1\r\n");

        let line = format_request_line(&request, RequestTarget::Authority);
        assert_eq!(&line, "GET example.com HTTP/1.1\r\n");

        let line = format_request_line(&request, RequestTarget::Asterisk);
        assert_eq!(&line, "GET * HTTP/1.1\r\n");
    }

    #[test]
    fn test_serialize_headers() {
        let mut headers = HeaderMap::new();
        headers.append("Name1", "hello world".try_into().unwrap());
        headers.append("Name1", "another value".try_into().unwrap());
        headers.append("Name2", "123".try_into().unwrap());

        let mut buffer = Vec::new();
        serialize_headers(&headers, &mut buffer);

        assert_eq!(
            buffer,
            b"name1: hello world\r\nname1: another value\r\nname2: 123\r\n"
        );
    }

    #[test]
    fn test_convert_parser_headers() {
        let mut parser_headers = [httparse::EMPTY_HEADER; 10];
        parser_headers[0].name = "name1";
        parser_headers[0].value = b"hello world";
        parser_headers[1].name = "name1";
        parser_headers[1].value = b"another value";
        parser_headers[2].name = "name2";
        parser_headers[2].value = b"123";

        let mut headers = HeaderMap::new();

        convert_parser_headers(&parser_headers, &mut headers).unwrap();
    }

    #[test]
    fn test_convert_parser_response() {
        let mut parser_headers = [httparse::EMPTY_HEADER; 10];
        parser_headers[0].name = "name1";
        parser_headers[0].value = b"hello world";
        parser_headers[1].name = "name1";
        parser_headers[1].value = b"another value";
        parser_headers[2].name = "name2";
        parser_headers[2].value = b"123";

        let parser_response = httparse::Response {
            version: Some(1),
            code: Some(404),
            reason: Some("not found!"),
            headers: &mut parser_headers,
        };

        let response = convert_parser_response(&parser_response).unwrap();

        assert_eq!(response.status(), 404);
        assert_eq!(
            response.headers().get("webaves-reason-phrase"),
            Some(&HeaderValue::from_static("not found!"))
        );
        assert_eq!(response.headers().len(), 4);
    }
}
