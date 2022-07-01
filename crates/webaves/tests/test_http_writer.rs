use std::io::{Cursor, Write};

use webaves::http::{MessageWriter, RequestHeader, ResponseHeader};

#[test_log::test]
fn test_write_request() {
    let dest = Cursor::new(Vec::new());
    let mut writer = MessageWriter::new(dest);

    let header = RequestHeader::new("GET", "/index.html");

    writer.begin_request(&header).unwrap();
    writer.write_body();
    writer.end_message().unwrap();

    let dest = writer.into_inner();

    assert_eq!(dest.get_ref(), b"GET /index.html HTTP/1.1\r\n\r\n");
}

#[test_log::test]
fn test_write_response() {
    let dest = Cursor::new(Vec::new());
    let mut writer = MessageWriter::new(dest);

    let mut header = ResponseHeader::new(200);
    header.status_line.reason_phrase = "OK".to_string();

    writer.begin_response(&header).unwrap();
    let body = writer.write_body();
    body.write_all(b"Hello world!").unwrap();
    writer.end_message().unwrap();

    let dest = writer.into_inner();

    assert_eq!(dest.get_ref(), b"HTTP/1.1 200 OK\r\n\r\nHello world!");
}
