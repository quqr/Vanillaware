use formats::codec::tegra_x1::*;

#[test]
fn identity_for_single_block() {
    // 1 个 4x4 块：块内 i=0 映射到 (0,0)，应原样拷贝；越界块被跳过。
    let mut pix = vec![0u8; 64];
    for (i, b) in pix.iter_mut().enumerate() {
        *b = i as u8;
    }
    let out = swizzle_16b(&pix, 4, 4);
    assert_eq!(&out[..64], &pix[..]);
}

#[test]
fn two_gobs_horizontal_for_wide_texture() {
    // 8 块宽 x 8 块高（32x32 像素）：第 32 个源块应落到 GOB 1（x=4）而非下一行。
    // 这对应 PHP 16 位图的第 2 个瓦片列，验证多 GOB 横向平铺。
    let mut pix = vec![0u8; 64 * 64];
    for (i, b) in pix.iter_mut().enumerate() {
        *b = (i / 64) as u8; // 每块填充自己的块号
    }
    let out = swizzle_16b(&pix, 32, 32);

    // 源块 32 → 目标块 (4, 0)：目标像素 (16..20, 0..4) 应全为 32。
    for r in 0..4 {
        for c in 16..20 {
            assert_eq!(out[(r * 32 + c) * 4], 32, "block 32 must land at (4,0)");
        }
    }
    // 源块 1 → 目标块 (0, 1)：目标像素 (0..4, 4..8) 应全为 1。
    for r in 4..8 {
        for c in 0..4 {
            assert_eq!(out[(r * 32 + c) * 4], 1, "block 1 must land at (0,1)");
        }
    }
}
