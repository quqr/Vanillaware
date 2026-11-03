pub const IDENTITY: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

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

pub fn scale4(sx: f64, sy: f64, sz: f64) -> [f64; 16] {
    let mut m = IDENTITY;
    m[0] = sx;
    m[5] = sy;
    m[10] = sz;
    m
}

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

/// Row-vector versions (transposes of the column-vector helpers above) so that
/// `p' = p · M` rotates the same way the engine does.
fn rotate_z_row(radian: f64) -> [f64; 16] {
    let sin = radian.sin();
    let cos = radian.cos();
    let mut m = IDENTITY;
    m[0] = cos;
    m[1] = sin;
    m[4] = -sin;
    m[5] = cos;
    m
}

fn rotate_y_row(radian: f64) -> [f64; 16] {
    let sin = radian.sin();
    let cos = radian.cos();
    let mut m = IDENTITY;
    m[0] = cos;
    m[2] = -sin;
    m[8] = sin;
    m[10] = cos;
    m
}

fn rotate_x_row(radian: f64) -> [f64; 16] {
    let sin = radian.sin();
    let cos = radian.cos();
    let mut m = IDENTITY;
    m[5] = cos;
    m[6] = sin;
    m[9] = -sin;
    m[10] = cos;
    m
}

/// Engine transform matrix, transcribed from `f_700058d708` + `f_700058e014`.
///
/// Layout is **row major with a row-vector convention** (`p' = p · M`), so the
/// translation lives in `m[12..15]`. The rotations are right-multiplied in the
/// order Z, X, Y, and the flip flags negate a whole column after the rotation
/// (`f_700058d708` negates `m[0] m[4] m[8] m[12]`, i.e. column 0).
pub fn engine_matrix(
    move_: &[f32],
    rotate: &[f32],
    scale: &[f32],
    flip_x: bool,
    flip_y: bool,
) -> [f64; 16] {
    let mut m = IDENTITY;
    m[0] = sx(scale);
    m[5] = sy(scale);

    m = mul44(&m, &rotate_z_row(rz(rotate)));
    m = mul44(&m, &rotate_x_row(rx(rotate)));
    m = mul44(&m, &rotate_y_row(ry(rotate)));

    m[12] = mv0(move_);
    m[13] = mv1(move_);
    m[14] = mv2(move_);

    if flip_x {
        for row in 0..4 {
            m[row * 4] = -m[row * 4];
        }
    }
    if flip_y {
        for row in 0..4 {
            m[row * 4 + 1] = -m[row * 4 + 1];
        }
    }
    m
}

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

fn round6(value: f32) -> f64 {
    (f64::from(value) * 1e6).round() / 1e6
}

fn sx(scale: &[f32]) -> f64 {
    round6(scale.first().copied().unwrap_or(0.0))
}

fn sy(scale: &[f32]) -> f64 {
    round6(scale.get(1).copied().unwrap_or(0.0))
}

fn rx(rotate: &[f32]) -> f64 {
    round6(rotate.first().copied().unwrap_or(0.0))
}

fn ry(rotate: &[f32]) -> f64 {
    round6(rotate.get(1).copied().unwrap_or(0.0))
}

fn rz(rotate: &[f32]) -> f64 {
    round6(rotate.get(2).copied().unwrap_or(0.0))
}

fn mv0(move_: &[f32]) -> f64 {
    round6(move_.first().copied().unwrap_or(0.0))
}

fn mv1(move_: &[f32]) -> f64 {
    round6(move_.get(1).copied().unwrap_or(0.0))
}

fn mv2(move_: &[f32]) -> f64 {
    round6(move_.get(2).copied().unwrap_or(0.0))
}
