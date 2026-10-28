use formats::mbs::quad::*;

#[test]
fn hexdec_ignores_non_hex() {
    assert_eq!(hexdec("0x0"), 0);
    assert_eq!(hexdec("0x5"), 5);
    assert_eq!(hexdec("0x1e"), 0x1e);
    assert_eq!(hexdec("0x820"), 0x820);
}

#[test]
fn bitflags_turn_into_names() {
    assert_eq!(
        bitflag_attr(0x5, "keyattr"),
        Attribute::Many(vec!["keyattr_1".to_string(), "keyattr_4".to_string(),])
    );
    assert_eq!(
        bitflag_attr(0x2, "keyattr"),
        Attribute::One("keyattr_2".to_string())
    );
}

#[test]
fn game_name_drops_console_prefix() {
    assert_eq!(game_name("swit unicorn overlord"), "unicorn overlord");
    assert_eq!(game_name("ps4 13 sentinels"), "13 sentinels");
}

#[test]
fn list_add_pads_with_zero() {
    let mut list: Vec<Pad<u8>> = Vec::new();
    set_at(&mut list, 2, Pad::Filled(7u8));
    assert_eq!(list, vec![Pad::Empty(0), Pad::Empty(0), Pad::Filled(7u8)]);
}
