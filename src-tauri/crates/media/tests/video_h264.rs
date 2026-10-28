use media::video::*;

const SPS_1080: &[u8] = &[
    0x67, 0x64, 0x00, 0x28, 0xac, 0xb6, 0x03, 0xc0, 0x11, 0x36, 0x02, 0x20, 0x00, 0x00, 0x0c,
    0x80, 0x00, 0x02, 0xed, 0x5c, 0x00, 0x00,
];

#[test]
fn parses_sps_dimensions() {
    let (width, height, profile, level) = parse_sps(SPS_1080).expect("the SPS must parse");
    assert_eq!((width, height), (1920, 1088));
    assert_eq!((profile, level), (100, 40));
}

#[test]
fn finds_nal_after_start_code() {
    let mut buffer = vec![0u8, 0, 0, 1, 0x09, 0x10, 0, 0, 1, 0x67, 0x64, 0x00, 0x28];
    buffer.extend_from_slice(&[0, 0, 1, 0x68, 0xee]);
    let sps = find_nal(&buffer, 7).expect("an SPS must be found");
    assert_eq!(sps[0] & 0x1f, 7);
    assert_eq!(sps.len(), 4);
}

#[test]
fn reads_exp_golomb() {
    // 0b1 -> 0, 0b010 -> 1, 0b011 -> 2
    let mut reader = BitReader::new(&[0b1_010_011_0]);
    assert_eq!(reader.ue(), Some(0));
    assert_eq!(reader.ue(), Some(1));
    assert_eq!(reader.ue(), Some(2));
}

#[test]
fn chunk_span_counts_the_first_eight_bytes() {
    assert_eq!(chunk_span(504), (512, 480));
    assert_eq!(chunk_span(280), (288, 256));
    assert_eq!(chunk_span(0x18), (32, 0));
    assert_eq!(chunk_span(500), (512, 476));
    assert_eq!(chunk_span(0x19), (64, 1));
}
