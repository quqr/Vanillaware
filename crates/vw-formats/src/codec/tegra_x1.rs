//! Tegra X1（Nintendo Switch）纹理块地址重排（de-swizzle）。
//!
//! 逐块移植自 rip-vanillaware 的 `FTEX_swit_unicorn.php`（`tegra_x1_swizzled_16_bits`
//! / `tegra_x1_swizzled_8_bits`）：GOB 瓦片平铺模型——
//! - 16 位（BC3/BC7）：4x8 像素块瓦片，块内映射 x=0x12 / y=0xd；
//! - 8 位（BC4）：8x8 像素块瓦片，块内映射 x=0x25 / y=0x1a；
//! - 瓦片先横（x 按 `tile_w` 步进）后纵（y 按 8 行步进），纵向每 0x80（128）块行
//!   重新分段；越过分段边界（`sy + y >= bh`）的瓦片不消费源块。
//!
//! 输入 `pix` 是按**编码块顺序**（每块 4x4 像素、行主序）排列的解码像素，
//! 输出是行主序整图。原 PHP 要求 `bw % tile_w == 0` 且 `bh % 8 == 0`（游戏数据
//! 均满足）；本实现以越界跳过代替报错，对异常尺寸做降级处理。

const TILE_16B: (i32, i32, i32) = (4, 0x12, 0xd);
const TILE_8B: (i32, i32, i32) = (8, 0x25, 0x1a);
/// 一个分段覆盖的块行数（PHP 的 `$sy += 0x80`）。
const SEG_ROWS: i32 = 0x80;

fn swizzle_bitmask(mut value: i32, mut mask: i32) -> i32 {
    let mut result = 0;
    let mut shift = 0;

    loop {
        if mask < 1 || value < 1 {
            break;
        }

        let bit_value = value & 1;
        let bit_mask = mask & 1;
        value >>= 1;
        mask >>= 1;

        if bit_mask != 0 {
            result |= bit_value << shift;
            shift += 1;
        }
    }

    result
}

/// 通用 GOB 瓦片重排。`ow`/`oh` 为补行后的像素尺寸（须为 4 的倍数）。
fn unswizzle(
    pix: &[u8],
    ow: i32,
    oh: i32,
    (tile_w, mask_x, mask_y): (i32, i32, i32),
    bpp: usize,
) -> Vec<u8> {
    let bw = (ow >> 2) as usize;
    let bh = (oh >> 2) as usize;
    let stride = bw * 4 * bpp;
    let mut dec = vec![0u8; stride * bh * 4];

    if bw == 0 || bh == 0 {
        return dec;
    }

    let tile_blocks = tile_w as usize * 8;
    let block_bytes = 16 * bpp;
    let mut src = 0usize;

    let mut sy = 0usize;
    while sy < bh {
        let mut x = 0usize;
        while x < bw {
            // PHP：y 每 8 行一瓦、最多到分段末尾；越过 bh 的瓦片不消费源块。
            let y_end = bh.min(sy + SEG_ROWS as usize);
            let mut y = sy;
            while y < y_end {
                if (src + 1) * block_bytes > pix.len() {
                    // 源块耗尽：后续瓦片同样无数据，提前结束。
                    return dec;
                }

                for i in 0..tile_blocks {
                    let bx = swizzle_bitmask(i as i32, mask_x) as usize + x;
                    let by = swizzle_bitmask(i as i32, mask_y) as usize + y;

                    if bx >= bw || by >= bh {
                        continue;
                    }

                    let s = (src + i) * block_bytes;
                    if s + block_bytes > pix.len() {
                        return dec;
                    }

                    // 每块 4 行、每行 4 像素，按行拷贝到目标块。
                    for r in 0..4 {
                        let dst = (by * 4 + r) * stride + bx * 4 * bpp;
                        let row = s + r * 4 * bpp;
                        dec[dst..dst + 4 * bpp].copy_from_slice(&pix[row..row + 4 * bpp]);
                    }
                }

                src += tile_blocks;
                y += 8;
            }

            x += tile_w as usize;
        }

        sy += SEG_ROWS as usize;
    }

    dec
}

/// 8 位（单通道）纹理解扭曲。返回 `ow * oh` 字节。
pub fn swizzle_8b(pix: &[u8], ow: i32, oh: i32) -> Vec<u8> {
    unswizzle(pix, ow, oh, TILE_8B, 1)
}

/// 16 位（RGBA8）纹理解扭曲。返回 `ow * oh * 4` 字节。
pub fn swizzle_16b(pix: &[u8], ow: i32, oh: i32) -> Vec<u8> {
    unswizzle(pix, ow, oh, TILE_16B, 4)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
