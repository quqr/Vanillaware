//! BC3 / BC4 解码。
//!
//! 逐行移植自 `VanillawareConverter/FTEXConverter/S3tcTexture.cs`。
//! **故意保留**原实现的偏差（例如 RGB565 的绿色通道没有回填低位、插值使用
//! `Math.Round` 的银行家舍入），否则无法与旧工具的产物逐字节对齐。

fn read_u16(data: &[u8], pos: usize) -> u16 {
    u16::from(data[pos]) | (u16::from(data[pos + 1]) << 8)
}

/// C# `Math.Round(double)` 默认是 MidpointRounding.ToEven。
fn round_half_even(v: f64) -> i32 {
    v.round_ties_even() as i32
}

fn clamp_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

fn bc1_index(data: &[u8], pos: usize) -> [usize; 16] {
    let mut index = [0usize; 16];
    for (p, i) in (pos..).zip(0..4) {
        let b = data[p];
        for j in (0..8).step_by(2) {
            index[i * 4 + j / 2] = usize::from((b >> j) & 3);
        }
    }
    index
}

fn bc3_index(data: &[u8], pos: usize) -> [usize; 16] {
    let mut index = [0usize; 16];
    let mut p = pos;
    for i in (0..6).step_by(3) {
        let b1 = u32::from(data[p]);
        let b2 = u32::from(data[p + 1]);
        let b3 = u32::from(data[p + 2]);
        p += 3;

        let int_val = b1 | (b2 << 8) | (b3 << 16);
        for j in (0..24).step_by(3) {
            index[i / 3 * 8 + j / 3] = ((int_val >> j) & 7) as usize;
        }
    }
    index
}

fn rgb565(int_val: u16) -> [u8; 4] {
    let r = ((int_val >> 8) & 0xf8) as u8;
    let g = ((int_val >> 3) & 0xfc) as u8;
    let b = ((int_val << 3) & 0xf8) as u8;
    [r, g, b, 255]
}

fn rgb_interpolate(c1: [u8; 4], f1: f64, c2: [u8; 4], f2: f64) -> [u8; 4] {
    let ch = |a: u8, b: u8| clamp_u8(round_half_even(f64::from(a) * f1 + f64::from(b) * f2));
    [ch(c1[0], c2[0]), ch(c1[1], c2[1]), ch(c1[2], c2[2]), 255]
}

fn bc3_alpha(data: &[u8], pos: usize) -> [i32; 16] {
    let mut a = [0i32; 8];
    a[0] = i32::from(data[pos]);
    a[1] = i32::from(data[pos + 1]);
    let index = bc3_index(data, pos + 2);

    if a[0] > a[1] {
        for (k, w) in [6.0, 5.0, 4.0, 3.0, 2.0, 1.0].iter().enumerate() {
            let v = f64::from(a[0]) * w / 7.0 + f64::from(a[1]) * (7.0 - w) / 7.0;
            a[2 + k] = round_half_even(v);
        }
    } else {
        for (k, w) in [4.0, 3.0, 2.0].iter().enumerate() {
            let v = f64::from(a[0]) * w / 5.0 + f64::from(a[1]) * (5.0 - w) / 5.0;
            a[2 + k] = round_half_even(v);
        }
        a[6] = 0;
        a[7] = 255;
    }

    let mut out = [0i32; 16];
    for i in 0..16 {
        out[i] = a[index[i]];
    }
    out
}

fn bc1_color(data: &[u8], pos: usize, bc: u8) -> [[u8; 4]; 16] {
    let int1 = read_u16(data, pos);
    let int2 = read_u16(data, pos + 2);
    let index = bc1_index(data, pos + 4);

    let mut rgb = [[0u8; 4]; 4];
    rgb[0] = rgb565(int1);
    rgb[1] = rgb565(int2);

    if bc != 1 || int1 > int2 {
        rgb[2] = rgb_interpolate(rgb[0], 2.0 / 3.0, rgb[1], 1.0 / 3.0);
        rgb[3] = rgb_interpolate(rgb[0], 1.0 / 3.0, rgb[1], 2.0 / 3.0);
    } else {
        rgb[2] = rgb_interpolate(rgb[0], 0.5, rgb[1], 0.5);
        rgb[3] = [0, 0, 0, 0];
    }

    let mut out = [[0u8; 4]; 16];
    for i in 0..16 {
        out[i] = rgb[index[i]];
    }
    out
}

/// BC3（= DXT5）：16 字节/块，输出 RGBA8，每块 16 像素 × 4 字节。
pub fn decode_bc3(data: &[u8]) -> Vec<u8> {
    let blocks = data.len() / 16;
    let mut out = Vec::with_capacity(blocks * 64);

    for b in 0..blocks {
        let pos = b * 16;
        if pos + 16 > data.len() {
            break;
        }

        let alpha = bc3_alpha(data, pos);
        let rgb = bc1_color(data, pos + 8, 3);

        for i in 0..16 {
            out.push(rgb[i][0]);
            out.push(rgb[i][1]);
            out.push(rgb[i][2]);
            out.push(clamp_u8(alpha[i]));
        }
    }

    out
}

/// BC4：8 字节/块，输出**单通道**（每像素 1 字节），与 C# 的 `Bc4` 一致。
pub fn decode_bc4(data: &[u8]) -> Vec<u8> {
    let blocks = data.len() / 8;
    let mut out = Vec::with_capacity(blocks * 16);

    for b in 0..blocks {
        let pos = b * 8;
        if pos + 8 > data.len() {
            break;
        }

        let alpha = bc3_alpha(data, pos);
        for value in alpha {
            out.push(clamp_u8(value));
        }
    }

    out
}
