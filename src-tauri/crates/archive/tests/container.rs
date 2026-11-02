mod common;

use std::io::Cursor;

use archive::{container, is_compressed, Table};

const SAMPLE: &str = "SampleData.cpk";
const SAMPLE_ENCRYPTED: &str = "SampleData-Encrypted.cpk";

#[test]
fn plain_header_table_parses() {
    let path = common::asset(SAMPLE);
    let bytes = std::fs::read(&path).expect("sample archive is missing");
    let mut reader = Cursor::new(&bytes);

    let header = container::read(&mut reader, 0, bytes.len() as u64).expect("container");
    assert_eq!(&header.magic, b"CPK ");
    assert!(!header.encrypted);

    let table = Table::parse(header.body).expect("table");
    assert_eq!(table.rows(), 1);
    assert_eq!(table.column_names().len(), 44);
    assert_eq!(table.row_width(), 130);
    assert_eq!(table.padding(), 0);
}

#[test]
fn encrypted_header_table_decrypts_to_the_known_plaintext() {
    let path = common::asset(SAMPLE_ENCRYPTED);
    let bytes = std::fs::read(&path).expect("encrypted sample archive is missing");
    let mut reader = Cursor::new(&bytes);

    let header = container::read(&mut reader, 0, bytes.len() as u64).expect("container");
    assert!(header.encrypted, "the XOR prefix was not detected");

    let plain = std::fs::read(common::asset("DecyptedTable.@utf")).expect("fixture is missing");
    assert_eq!(header.body, plain);
}

#[test]
fn container_length_is_little_endian() {
    let path = common::asset(SAMPLE);
    let bytes = std::fs::read(&path).expect("sample archive is missing");

    let big_endian = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let little_endian = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    assert_ne!(big_endian, little_endian);
    assert_eq!(little_endian, 856);

    let mut reader = Cursor::new(&bytes);
    let header = container::read(&mut reader, 0, bytes.len() as u64).expect("container");
    assert_eq!(header.body.len(), 856);
}

#[test]
fn implausible_table_length_is_rejected() {
    let mut bytes = std::fs::read(common::asset(SAMPLE)).expect("sample archive is missing");
    bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());

    let mut reader = Cursor::new(&bytes);
    let error = container::read(&mut reader, 0, bytes.len() as u64).expect_err("must reject");
    assert!(error.to_string().contains("implausible"));
}

#[test]
fn compressed_text_round_trips_against_the_expected_output() {
    let packed = std::fs::read(common::asset("CompressedText.crilayla")).expect("fixture missing");
    assert!(is_compressed(&packed));

    let out = archive::decompress(&packed).expect("decompress");
    let expected =
        std::fs::read(common::asset("SampleData/Text-Compressed.txt")).expect("fixture missing");
    assert_eq!(out.len(), 3592);
    assert_eq!(out, expected);
}

#[test]
fn compressed_model_round_trips_to_the_declared_length() {
    let packed =
        std::fs::read(common::asset("Compressed3dModel.crilayla")).expect("fixture missing");
    let out = archive::decompress(&packed).expect("decompress");
    assert_eq!(out.len(), 169637);
}
