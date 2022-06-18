use std::{fs::File, io::Read, path::PathBuf};

use webaves::warc::WARCReader;

#[test_log::test]
fn minimal_warc_read() {
    let path = [env!("CARGO_MANIFEST_DIR"), "tests/warc_minimal.warc"]
        .iter()
        .collect::<PathBuf>();
    dbg!(&path);
    let file = File::open(path).unwrap();
    let mut reader = WARCReader::new(file).unwrap();

    // record 0
    let metadata = reader.begin_record().unwrap().unwrap();

    assert_eq!(metadata.version(), "WARC/1.1");
    assert_eq!(metadata.file_offset(), 0);
    assert_eq!(metadata.block_length(), 10);
    assert_eq!(
        metadata.header().get_str("WARC-Record-ID").unwrap(),
        "<urn:uuid:00000001-0002-0003-0004-000000000005>"
    );

    let mut block_buf = Vec::new();
    let mut block_reader = reader.read_block();
    block_reader.read_to_end(&mut block_buf).unwrap();

    assert_eq!(block_buf.len(), 10);
    assert_eq!(block_buf, b"\xf0\xf1\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9");

    reader.end_record(block_reader).unwrap();

    // record 1
    let metadata = reader.begin_record().unwrap().unwrap();

    assert_eq!(metadata.version(), "WARC/1.1");
    assert_eq!(metadata.file_offset(), 165);
    assert_eq!(metadata.block_length(), 16);
    assert_eq!(
        metadata.header().get_str("WARC-Record-ID").unwrap(),
        "<urn:uuid:10000001-0002-0003-0004-000000000005>"
    );

    let mut block_buf = Vec::new();
    let mut block_reader = reader.read_block();
    block_reader.read_to_end(&mut block_buf).unwrap();

    assert_eq!(block_buf.len(), 16);
    assert_eq!(
        block_buf,
        b"\xf0\xf1\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9\xfa\xfb\xfc\xfd\xfe\xff"
    );

    reader.end_record(block_reader).unwrap();

    // eof
    let result = reader.begin_record().unwrap();
    assert!(result.is_none());
}