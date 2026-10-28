use media::hash::*;

#[test]
fn stable_and_lowercase_sensitive() {
    let a = fnv1a64_path(r"f:\codes\a\b.ftx");
    let b = fnv1a64_path(r"f:\codes\a\b.ftx");
    assert_eq!(a, b, "the same path must hash stably");
    assert_eq!(a.len(), 16);

    assert_eq!(a, fnv1a64_path(r"F:\CODES\A\B.FTX"));

    assert_ne!(a, fnv1a64_path(r"f:\codes\a\b1.ftx"));
}
