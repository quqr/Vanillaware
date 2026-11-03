use formats::atlas::*;

#[test]
fn picks_sibling_by_extension_priority() {
    let dir = std::env::temp_dir().join("atlas-sibling-test");
    std::fs::create_dir_all(&dir).expect("create a temp directory");
    let mbs = dir.join("Sample.mbs");
    std::fs::write(&mbs, b"").expect("write mbs");

    assert_eq!(sibling_container(&mbs).expect("parse"), None);

    let ftp = dir.join("Sample.ftp");
    std::fs::write(&ftp, b"").expect("write ftp");
    assert_eq!(
        sibling_container(&mbs).expect("parse").as_deref(),
        Some(ftp.as_path())
    );

    let ftx = dir.join("Sample.ftx");
    std::fs::write(&ftx, b"").expect("write ftx");
    assert_eq!(
        sibling_container(&mbs).expect("parse").as_deref(),
        Some(ftx.as_path())
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn falls_back_to_numeric_suffix_variants() {
    let dir = std::env::temp_dir().join("atlas-suffix-test");
    std::fs::create_dir_all(&dir).expect("create a temp directory");
    let mbs = dir.join("Fighter_M.mbs");
    std::fs::write(&mbs, b"").expect("write mbs");

    let variant = dir.join("Fighter_M00.ftx");
    std::fs::write(&variant, b"").expect("write a variant");
    assert_eq!(
        sibling_container(&mbs).expect("parse").as_deref(),
        Some(variant.as_path())
    );

    std::fs::write(dir.join("Fighter_M10.ftx"), b"").expect("write a second variant");
    assert!(
        sibling_container(&mbs).is_err(),
        "several candidates must fail instead of guessing"
    );

    std::fs::remove_dir_all(&dir).ok();
}
