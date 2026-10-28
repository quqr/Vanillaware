use vw_media::video::*;

/// 1920×1080 的一段真实 SPS（profile 100 / level 40，带 crop）。
const SPS_1080: &[u8] = &[
    0x67, 0x64, 0x00, 0x28, 0xac, 0xb6, 0x03, 0xc0, 0x11, 0x36, 0x02, 0x20, 0x00, 0x00, 0x0c,
    0x80, 0x00, 0x02, 0xed, 0x5c, 0x00, 0x00,
];

#[test]
fn parses_sps_dimensions() {
    let (width, height, profile, level) = parse_sps(SPS_1080).expect("SPS 应当可解");
    // 编码高度按 16 对齐，1080p 的 SPS 里就是 1088 且没有 crop。
    assert_eq!((width, height), (1920, 1088));
    assert_eq!((profile, level), (100, 40));
}

#[test]
fn finds_nal_after_start_code() {
    let mut buffer = vec![0u8, 0, 0, 1, 0x09, 0x10, 0, 0, 1, 0x67, 0x64, 0x00, 0x28];
    buffer.extend_from_slice(&[0, 0, 1, 0x68, 0xee]);
    let sps = find_nal(&buffer, 7).expect("应当找到 SPS");
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

/// `size` 不含块头前 8 字节；语料里 `size % 32 == 24`，载荷就是 `size - 0x18`。
#[test]
fn chunk_span_counts_the_first_eight_bytes() {
    assert_eq!(chunk_span(504), (512, 480));
    assert_eq!(chunk_span(280), (288, 256));
    assert_eq!(chunk_span(0x18), (32, 0));
    // 语料之外的尺寸：块长仍对齐，载荷只按 size 取，不把填充算进码流。
    assert_eq!(chunk_span(500), (512, 476));
    // size+8 跨过 0x20 边界时整块涨到下一个 0x20，载荷仍只取 1 字节。
    assert_eq!(chunk_span(0x19), (64, 1));
}
