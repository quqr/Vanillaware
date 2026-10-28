use formats::codec::tegra_x1::*;

#[test]
fn identity_for_single_block() {
    let mut pix = vec![0u8; 64];
    for (i, b) in pix.iter_mut().enumerate() {
        *b = i as u8;
    }
    let out = swizzle_16b(&pix, 4, 4);
    assert_eq!(&out[..64], &pix[..]);
}

#[test]
fn two_gobs_horizontal_for_wide_texture() {
    let mut pix = vec![0u8; 64 * 64];
    for (i, b) in pix.iter_mut().enumerate() {
        *b = (i / 64) as u8;
    }
    let out = swizzle_16b(&pix, 32, 32);

    for r in 0..4 {
        for c in 16..20 {
            assert_eq!(out[(r * 32 + c) * 4], 32, "block 32 must land at (4,0)");
        }
    }
    for r in 4..8 {
        for c in 0..4 {
            assert_eq!(out[(r * 32 + c) * 4], 1, "block 1 must land at (0,1)");
        }
    }
}
