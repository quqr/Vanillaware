use formats::mbs::matrix::engine_matrix;

fn apply(m: &[f64; 16], x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[4] * y + m[12], m[1] * x + m[5] * y + m[13])
}

fn close(actual: (f64, f64), expected: (f64, f64)) {
    assert!(
        (actual.0 - expected.0).abs() < 1e-6 && (actual.1 - expected.1).abs() < 1e-6,
        "{actual:?} != {expected:?}"
    );
}

/// `f_700058d708` writes the translation into `m[12..15]`, so the matrix is row
/// major with a row-vector convention.
#[test]
fn translation_lives_in_the_last_row() {
    let m = engine_matrix(&[3.0, 4.0, 5.0], &[0.0; 3], &[1.0, 1.0], false, false);
    close(apply(&m, 0.0, 0.0), (3.0, 4.0));
    assert_eq!(m[3], 0.0);
    assert_eq!(m[7], 0.0);
    assert_eq!(m[11], 0.0);
    assert_eq!(m[15], 1.0);
}

/// `f_700058e014` rotates around Z, then X, then Y, each right-multiplied.
#[test]
fn rotation_order_is_z_then_x_then_y() {
    let half = std::f32::consts::FRAC_PI_2;

    let rz = engine_matrix(&[0.0; 3], &[0.0, 0.0, half], &[1.0, 1.0], false, false);
    close(apply(&rz, 1.0, 0.0), (0.0, 1.0));

    let rx = engine_matrix(&[0.0; 3], &[half, 0.0, 0.0], &[1.0, 1.0], false, false);
    close(apply(&rx, 0.0, 1.0), (0.0, 0.0));

    let ry = engine_matrix(&[0.0; 3], &[0.0, half, 0.0], &[1.0, 1.0], false, false);
    close(apply(&ry, 1.0, 0.0), (0.0, 0.0));

    // X before Z would collapse the point to the origin instead of rotating it.
    let both = engine_matrix(&[0.0; 3], &[half, 0.0, half], &[1.0, 1.0], false, false);
    close(apply(&both, 0.0, 1.0), (-1.0, 0.0));
}

/// `f_700058d708` seeds the matrix with the scale and only then calls the
/// rotation helper, so the scale must run first.
#[test]
fn scale_runs_before_the_rotation() {
    let half = std::f32::consts::FRAC_PI_2;
    let m = engine_matrix(&[0.0; 3], &[0.0, 0.0, half], &[2.0, 1.0], false, false);
    close(apply(&m, 1.0, 0.0), (0.0, 2.0));
}

/// The frame flags negate a whole column, including the translation entry.
#[test]
fn flip_flags_mirror_a_column() {
    let base = engine_matrix(&[0.0; 3], &[0.0; 3], &[1.0, 1.0], false, false);
    close(apply(&base, 2.0, 3.0), (2.0, 3.0));

    let flip_x = engine_matrix(&[0.0; 3], &[0.0; 3], &[1.0, 1.0], true, false);
    close(apply(&flip_x, 2.0, 3.0), (-2.0, 3.0));

    let flip_y = engine_matrix(&[0.0; 3], &[0.0; 3], &[1.0, 1.0], false, true);
    close(apply(&flip_y, 2.0, 3.0), (2.0, -3.0));

    let both = engine_matrix(&[5.0, 6.0, 0.0], &[0.0; 3], &[1.0, 1.0], true, true);
    close(apply(&both, 0.0, 0.0), (-5.0, -6.0));
}
