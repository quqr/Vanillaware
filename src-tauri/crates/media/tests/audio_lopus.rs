use media::audio::*;

fn build_lopus() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&CHUNK_BASIC.to_le_bytes());
    bytes.extend_from_slice(&0x18u32.to_le_bytes());
    bytes.extend_from_slice(&[0x00, 0x02, 0xf8, 0x00]); // version, channels, frame_size
    bytes.extend_from_slice(&48000u32.to_le_bytes());
    bytes.extend_from_slice(&0x20u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&120u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());

    let frame_size = 248usize;
    let frames = 2usize;
    bytes.extend_from_slice(&CHUNK_DATA.to_le_bytes());
    bytes.extend_from_slice(&((frame_size * frames) as u32).to_le_bytes());
    for frame in 0..frames {
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0xf0, frame as u8, 0x78, 0x5a, 0x00]);
        bytes.extend(std::iter::repeat_n(0xfc, frame_size - LOPUS_FRAME_HEADER));
    }
    bytes
}

#[test]
fn lopus_splits_fixed_frames() {
    let clip = parse_lopus(&build_lopus()).expect("must parse");
    match clip {
        Clip::OpusPackets {
            sample_rate,
            channels,
            stride,
            skip,
            packets,
            frames,
        } => {
            assert_eq!(sample_rate, 48000);
            assert_eq!(channels, 2);
            assert_eq!(stride, 240);
            assert_eq!(skip, 120);
            assert_eq!(frames, 1920);
            assert_eq!(packets.len(), 480);
            assert_eq!(packets[0], 0xfc);
        }
        _ => panic!("must be detected as Opus"),
    }
}

#[test]
fn lopus_rejects_bad_magic() {
    let mut bytes = build_lopus();
    bytes[0] = 0;
    assert!(parse_lopus(&bytes).is_err());
}

#[test]
fn lopus_tolerates_alignment_padding() {
    let mut bytes = vec![0u8; 8];
    bytes.extend_from_slice(&build_lopus());
    let clip = parse_lopus(&bytes).expect("leading zeros must not count as corruption");
    match clip {
        Clip::OpusPackets {
            sample_rate,
            stride,
            frames,
            ..
        } => {
            assert_eq!(sample_rate, 48000);
            assert_eq!(stride, 240);
            assert_eq!(frames, 1920);
        }
        _ => panic!("must be detected as Opus"),
    }
}

#[test]
fn codec_sniffs_headers() {
    assert_eq!(Codec::sniff(b"HCA\0\x03\x00\x00\x60"), Codec::Hca);
    assert_eq!(
        Codec::sniff(&[0x01, 0x00, 0x00, 0x80, 0x18, 0x00, 0x00, 0x00]),
        Codec::Lopus
    );
    assert_eq!(Codec::from_wave_ext(".lopus"), Codec::Lopus);
    assert_eq!(Codec::from_wave_ext("hca"), Codec::Hca);
}

#[test]
fn data_header_overrides_declared_codec() {
    let hca = b"HCA\0\x03\x00\x00\x60";
    assert_eq!(effective_codec(&Codec::Lopus, hca), Codec::Hca);
    let lopus = [0x01, 0x00, 0x00, 0x80, 0x18, 0x00, 0x00, 0x00];
    assert_eq!(effective_codec(&Codec::Hca, &lopus), Codec::Lopus);
    assert_eq!(effective_codec(&Codec::Adx, b"????"), Codec::Adx);
}
