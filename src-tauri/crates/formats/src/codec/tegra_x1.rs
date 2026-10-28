const TILE_16B: (i32, i32, i32) = (4, 0x12, 0xd);
const TILE_8B: (i32, i32, i32) = (8, 0x25, 0x1a);
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
            let y_end = bh.min(sy + SEG_ROWS as usize);
            let mut y = sy;
            while y < y_end {
                if (src + 1) * block_bytes > pix.len() {
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

pub fn swizzle_8b(pix: &[u8], ow: i32, oh: i32) -> Vec<u8> {
    unswizzle(pix, ow, oh, TILE_8B, 1)
}

pub fn swizzle_16b(pix: &[u8], ow: i32, oh: i32) -> Vec<u8> {
    unswizzle(pix, ow, oh, TILE_16B, 4)
}
