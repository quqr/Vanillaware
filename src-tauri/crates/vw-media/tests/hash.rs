use vw_media::hash::*;

#[test]
fn stable_and_lowercase_sensitive() {
    let a = fnv1a64_path(r"f:\codes\a\b.ftx");
    let b = fnv1a64_path(r"f:\codes\a\b.ftx");
    assert_eq!(a, b, "同一路径必须稳定");
    assert_eq!(a.len(), 16);

    // Windows 大小写不敏感：key 必须一致
    assert_eq!(a, fnv1a64_path(r"F:\CODES\A\B.FTX"));

    // 真正不同的路径不该撞（这两串只在大小写以外不同）
    assert_ne!(a, fnv1a64_path(r"f:\codes\a\b1.ftx"));
}
