use vw_formats::mbs::profile::*;

/// v77 与 v76/v72 的关键差异：s0..s4 的 count 宽度为 4，s4 步长相异。
#[test]
fn version_differences() {
    let v77 = lookup("swi_unic").expect("swi_unic");
    let v76 = lookup("ps4_sent").expect("ps4_sent");
    let v72 = lookup("ps4_odin").expect("ps4_odin");

    assert_eq!(v77.sections[0].count_bytes, 4);
    assert_eq!(v76.sections[0].count_bytes, 2);

    assert_eq!(v77.sections[4].stride, 0x14);
    assert_eq!(v76.sections[4].stride, 0x14);
    assert_eq!(v72.sections[4].stride, 0x10);

    assert!(lookup("ps3_odin").expect("ps3_odin").big_endian);
    assert!(!v77.big_endian);

    assert!(lookup("nds_kuma").is_none(), "ver66 不在本轮范围");
}

/// `kind` 决定 s4 走哪份记录定义，`version` 决定自校验的期望值。
#[test]
fn kind_and_version_track_generation() {
    let v77 = lookup("swi_unic").expect("swi_unic");
    let v76 = lookup("ps4_sent").expect("ps4_sent");
    let v72 = lookup("ps3_odin").expect("ps3_odin");

    assert_eq!((v72.kind, v72.version), (0, 0x72));
    assert_eq!((v76.kind, v76.version), (1, 0x76));
    assert_eq!((v77.kind, v77.version), (2, 0x77));

    assert_eq!(v77.id3, "swit unicorn overlord");
    assert_eq!(v72.id3, "ps3 odin sphere");
}

/// count 的读取宽度随版本变化：v77 的 s0 是 4 字节，v76 是 2 字节。
#[test]
fn read_counts_respects_width() {
    let mut data = vec![0u8; 0x120];
    // v77 s0 的 count 在 0x54，宽 4。
    data[0x54..0x58].copy_from_slice(&7u32.to_le_bytes());
    // v77 s5 的 count 在 0x64，宽 2。
    data[0x64..0x66].copy_from_slice(&3u16.to_le_bytes());

    let v77 = lookup("swi_unic").expect("swi_unic");
    let counts = v77.read_counts(&data).expect("counts");
    assert_eq!(counts[0], 7);
    assert_eq!(counts[5], 3);

    assert!(v77.read_counts(&data[..0x40]).is_err(), "截断文件应报错");
}
