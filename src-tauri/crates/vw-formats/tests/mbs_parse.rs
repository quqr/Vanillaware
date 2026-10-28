use vw_formats::mbs::parse::*;
use vw_formats::mbs::v55::FogQuad;

#[test]
fn quad8_takes_indices_2_to_9() {
    let floats: Vec<f32> = (0..12).map(|i| i as f32).collect();
    assert_eq!(quad8(&floats), vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
}

#[test]
fn fog_quad_degrades_only_when_all_equal() {
    let same: Vec<u8> = vec![0xff, 0x00, 0x12, 0x34];
    let other: Vec<u8> = vec![0xff, 0x00, 0x12, 0x35];
    assert_eq!(
        fog_quad([&same, &same, &same, &same]),
        FogQuad::One("#ff001234".into())
    );
    assert_eq!(
        fog_quad([&same, &same, &same, &other]),
        FogQuad::Four([
            "#ff001234".into(),
            "#ff001234".into(),
            "#ff001234".into(),
            "#ff001235".into()
        ])
    );
}

#[test]
fn c_string_stops_at_first_nul() {
    assert_eq!(c_string(b"abc\0def"), "abc");
    assert_eq!(c_string(b"abcdef"), "abcdef");
    assert_eq!(c_string(b"a\0\0"), "a");
}
