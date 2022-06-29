use std::{fs::File, io::Read, path::PathBuf};

use webaves::{http::MessageReader, io::ComboReader};

#[test_log::test]
fn test_read_requests() {
    let path = [env!("CARGO_MANIFEST_DIR"), "tests/http_request_minimal"]
        .iter()
        .collect::<PathBuf>();

    let file = File::open(path).unwrap();
    let mut reader = MessageReader::new(ComboReader::new(file));
    let mut body = Vec::new();

    // GET
    let header = reader.begin_request().unwrap();
    assert_eq!(header.request_line.method, "GET");
    assert_eq!(header.request_line.target, "/index.html");
    assert_eq!(header.fields.get_str("host"), Some("example.com"));

    let body_reader = reader.read_body();
    body.clear();
    body_reader.read_to_end(&mut body).unwrap();
    assert_eq!(body, b"");

    reader.end_message().unwrap();

    // POST

    let header = reader.begin_request().unwrap();

    assert_eq!(header.request_line.method, "POST");
    assert_eq!(header.request_line.target, "/api");
    assert_eq!(header.fields.get_str("host"), Some("example.com"));

    let body_reader = reader.read_body();
    body.clear();
    body_reader.read_to_end(&mut body).unwrap();
    assert_eq!(body, b"Hello world!\r\n");

    reader.end_message().unwrap();
}

#[test_log::test]
fn test_read_responses() {
    let path = [env!("CARGO_MANIFEST_DIR"), "tests/http_response_minimal"]
        .iter()
        .collect::<PathBuf>();

    let file = File::open(path).unwrap();
    let mut reader = MessageReader::new(ComboReader::new(file));
    let mut body = Vec::new();

    // Content length

    let header = reader.begin_response(None).unwrap();
    assert_eq!(header.status_line.status_code, 200);

    let body_reader = reader.read_body();
    body.clear();
    body_reader.read_to_end(&mut body).unwrap();
    assert_eq!(body, b"Hello world!\r\n");

    reader.end_message().unwrap();

    // Chunked

    let header = reader.begin_response(None).unwrap();
    assert_eq!(header.status_line.status_code, 200);

    let body_reader = reader.read_body();
    body.clear();
    body_reader.read_to_end(&mut body).unwrap();

    assert_eq!(body, b"Hello world!");

    reader.end_message().unwrap();

    // No content length (legacy)

    let header = reader.begin_response(None).unwrap();
    assert_eq!(header.status_line.status_code, 200);

    let body_reader = reader.read_body();
    body.clear();
    body_reader.read_to_end(&mut body).unwrap();

    assert_eq!(body, b"Hello world!\r\n");

    reader.end_message().unwrap();
}
