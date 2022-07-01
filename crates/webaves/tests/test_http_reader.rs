use std::{
    fs::File,
    io::{Cursor, Read},
    path::PathBuf,
};

use webaves::{http::MessageReader, io::ComboReader};

#[test_log::test]
fn test_read_requests() {
    let path = PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/http_request_minimal");

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
    let path = PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/http_response_minimal");

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

#[test_log::test]
fn test_read_response_gzip() {
    let path = PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/http_response_gzip");
    let gzip_path = PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/quick_brown_fox.gz");

    let file = File::open(path).unwrap();
    let gzip_file = File::open(gzip_path).unwrap();
    let data = file.take(89).chain(gzip_file);

    let mut reader = MessageReader::new(ComboReader::new(data));
    let mut body = Vec::new();

    let header = reader.begin_response(None).unwrap();
    assert_eq!(header.status_line.status_code, 200);

    let body_reader = reader.read_body();
    body.clear();
    body_reader.read_to_end(&mut body).unwrap();
    assert_eq!(body, b"The quick brown fox jumps over the lazy dog.");

    reader.end_message().unwrap();
}

#[test_log::test]
fn test_read_response_zero_nine() {
    let data = Cursor::new(b"Hello world!\r\n");

    let mut reader = MessageReader::new(ComboReader::new(data));
    let mut body = Vec::new();

    let header = reader.begin_response(None).unwrap();
    assert_eq!(header.status_line.version, (0, 9));

    let body_reader = reader.read_body();
    body.clear();
    body_reader.read_to_end(&mut body).unwrap();
    assert_eq!(body, b"Hello world!\r\n");

    reader.end_message().unwrap();
}
