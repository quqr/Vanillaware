use formats::mbs::matrix::*;

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
