//! PHP `func-matrix.inc` 中 4×4 部分的逐行移植（ADR-0004：真值是 PHP）。
//!
//! 矩阵是展平的**行主序** `[f64; 16]`，下标为 `y * 4 + x`；[`mul44`] 即 `m1 × m2`。
//! PHP 全程用 f64，本模块保持 f64，避免 `sin` / `cos` 的误差与参考实现分叉。
//!
//! 唯一有意保留的差异：PHP 用 `implode(',', $m)` 的**字符串**判等来识别单位矩阵
//! （受 `precision=14` 影响）；本模块用逐元素精确比较。触发条件需要浮点误差恰好落在
//! 第 14 位有效数字之下，且此时 PHP 侧的 `sin`/`cos` 亦已被字符串化，实际不可达。

/// 4×4 单位矩阵（行主序），对应 PHP `matrix::new(4)`。
pub const IDENTITY: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// 对应 PHP `matrix::multi44`：`m1 × m2`，行主序。
pub fn mul44(m1: &[f64; 16], m2: &[f64; 16]) -> [f64; 16] {
    let mut out = [0.0f64; 16];
    for y in 0..4 {
        for x in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += m1[y * 4 + k] * m2[k * 4 + x];
            }
            out[y * 4 + x] = sum;
        }
    }
    out
}

/// 对应 PHP `matrix::scale(4, sx, sy, sz)`；`sz` 缺省为 1。
pub fn scale4(sx: f64, sy: f64, sz: f64) -> [f64; 16] {
    let mut m = IDENTITY;
    m[0] = sx;
    m[5] = sy;
    m[10] = sz;
    m
}

/// 对应 PHP `matrix::rotate_z(4, $radian)`。
pub fn rotate_z4(radian: f64) -> [f64; 16] {
    let sin = radian.sin();
    let cos = radian.cos();
    let mut m = IDENTITY;
    m[0] = cos;
    m[1] = -sin;
    m[4] = sin;
    m[5] = cos;
    m
}

/// 对应 PHP `matrix::rotate_y(4, $radian)`。
pub fn rotate_y4(radian: f64) -> [f64; 16] {
    let sin = radian.sin();
    let cos = radian.cos();
    let mut m = IDENTITY;
    m[0] = cos;
    m[2] = sin;
    m[8] = -sin;
    m[10] = cos;
    m
}

/// 对应 PHP `matrix::rotate_x(4, $radian)`。
pub fn rotate_x4(radian: f64) -> [f64; 16] {
    let sin = radian.sin();
    let cos = radian.cos();
    let mut m = IDENTITY;
    m[5] = cos;
    m[6] = -sin;
    m[9] = sin;
    m[10] = cos;
    m
}

/// 对应 PHP `s7_matrix($s7, $flipx, $flipy)`。
///
/// 合成顺序为 scale - rotate z - rotate y - rotate x - move（与 PHP 一致）。
/// 平移**直接加到第 4 列**（`m[3]` / `m[7]` / `m[11]`），不是右乘一个平移矩阵——
/// 这是 PHP 与旧 C# 工具的算法级差异之一（ADR-0006 第 1 项）。
///
/// 翻转因子 `bx` / `by` **同时乘在缩放与平移上**（PHP：`scale($sx*$bx, $sy*$by)` 与
/// `move[0]*$bx` / `move[1]*$by`），只乘平移会漏掉缩放行的符号。
///
/// 入参是 `.v55` 里的 f32，读出来先经 [`round6`] 收敛到 6 位小数：PHP 的转换器读的是
/// **自己写出的 `.v55` JSON**，那里的浮点已被 `jsonpretty` 的 `number_format(x, 6)`
/// 收敛过；不收敛的话，缩放与旋转的乘积会把 f32 的舍入误差放大到 `1e-6` 容差之外。
///
/// 返回空数组表示单位矩阵（PHP 返回 `[]`）。
pub fn s7_matrix(
    move_: &[f32],
    rotate: &[f32],
    scale: &[f32],
    flip_x: bool,
    flip_y: bool,
) -> Vec<f64> {
    let bx = if flip_x { -1.0 } else { 1.0 };
    let by = if flip_y { -1.0 } else { 1.0 };

    let mut m = scale4(sx(scale) * bx, sy(scale) * by, 1.0);
    m = mul44(&m, &rotate_z4(rz(rotate)));
    if ry(rotate) != 0.0 {
        m = mul44(&m, &rotate_y4(ry(rotate)));
    }
    if rx(rotate) != 0.0 {
        m = mul44(&m, &rotate_x4(rx(rotate)));
    }

    m[3] += mv0(move_) * bx;
    m[7] += mv1(move_) * by;
    m[11] += mv2(move_);

    if m == IDENTITY {
        return Vec::new();
    }
    m.to_vec()
}

/// 对齐 PHP `jsonpretty` 的 `number_format(0.0 + $val, 6, '.', '')`：收敛到 6 位小数。
///
/// PHP 的 `number_format` 是「四舍五入、远离零」，与 Rust `f64::round` 一致。
fn round6(value: f32) -> f64 {
    (f64::from(value) * 1e6).round() / 1e6
}

/// s7 的 `scale[0]`；缺项按 0 处理（PHP 越界取 null 会以 0 参与运算）。
fn sx(scale: &[f32]) -> f64 {
    round6(scale.first().copied().unwrap_or(0.0))
}

/// s7 的 `scale[1]`。
fn sy(scale: &[f32]) -> f64 {
    round6(scale.get(1).copied().unwrap_or(0.0))
}

/// s7 的 `rotate[0]`（绕 x 轴）。
fn rx(rotate: &[f32]) -> f64 {
    round6(rotate.first().copied().unwrap_or(0.0))
}

/// s7 的 `rotate[1]`（绕 y 轴）。
fn ry(rotate: &[f32]) -> f64 {
    round6(rotate.get(1).copied().unwrap_or(0.0))
}

/// s7 的 `rotate[2]`（绕 z 轴）。
fn rz(rotate: &[f32]) -> f64 {
    round6(rotate.get(2).copied().unwrap_or(0.0))
}

/// s7 的 `move[0]`。
fn mv0(move_: &[f32]) -> f64 {
    round6(move_.first().copied().unwrap_or(0.0))
}

/// s7 的 `move[1]`。
fn mv1(move_: &[f32]) -> f64 {
    round6(move_.get(1).copied().unwrap_or(0.0))
}

/// s7 的 `move[2]`。
fn mv2(move_: &[f32]) -> f64 {
    round6(move_.get(2).copied().unwrap_or(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_matrix_yields_empty() {
        let m = s7_matrix(
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0],
            &[1.0, 1.0],
            false,
            false,
        );
        assert!(m.is_empty());
    }

    #[test]
    fn flip_and_move_degrade_identity() {
        let m = s7_matrix(&[1.0, 2.0, 3.0], &[0.0; 3], &[1.0, 1.0], false, false);
        assert_eq!(m.len(), 16);
        assert_eq!(m[3], 1.0);
        assert_eq!(m[7], 2.0);
        assert_eq!(m[11], 3.0);
    }

    #[test]
    fn scale_rotate_z_composes_like_php() {
        // 缩放 2 倍再绕 z 轴 90 度：sin=1、cos≈0。
        let m = s7_matrix(
            &[0.0; 3],
            &[0.0, 0.0, std::f32::consts::FRAC_PI_2],
            &[2.0, 2.0],
            false,
            false,
        );
        assert!((m[0]).abs() < 1e-6);
        assert!((m[1] + 2.0).abs() < 1e-6);
        assert!((m[4] - 2.0).abs() < 1e-6);
        assert!((m[5]).abs() < 1e-6);
    }
}
