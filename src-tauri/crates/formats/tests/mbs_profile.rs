use formats::mbs::profile::*;

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

    assert!(lookup("nds_kuma").is_none(), "ver66 is out of scope here");
}

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

#[test]
fn read_counts_respects_width() {
    let mut data = vec![0u8; 0x120];
    data[0x54..0x58].copy_from_slice(&7u32.to_le_bytes());
    data[0x64..0x66].copy_from_slice(&3u16.to_le_bytes());

    let v77 = lookup("swi_unic").expect("swi_unic");
    let counts = v77.read_counts(&data).expect("counts");
    assert_eq!(counts[0], 7);
    assert_eq!(counts[5], 3);

    assert!(
        v77.read_counts(&data[..0x40]).is_err(),
        "a truncated file must fail"
    );
}
