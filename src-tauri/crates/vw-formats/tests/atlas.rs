use vw_formats::atlas::*;

/// 同名 `.ftx` 优先于同名 `.ftp`；都没有时退回「同名 + 数字后缀」的唯一候选。
#[test]
fn picks_sibling_by_extension_priority() {
    let dir = std::env::temp_dir().join("vw_atlas_sibling_test");
    std::fs::create_dir_all(&dir).expect("创建临时目录");
    let mbs = dir.join("Sample.mbs");
    std::fs::write(&mbs, b"").expect("写 mbs");

    assert_eq!(sibling_container(&mbs).expect("解析"), None);

    let ftp = dir.join("Sample.ftp");
    std::fs::write(&ftp, b"").expect("写 ftp");
    assert_eq!(
        sibling_container(&mbs).expect("解析").as_deref(),
        Some(ftp.as_path())
    );

    let ftx = dir.join("Sample.ftx");
    std::fs::write(&ftx, b"").expect("写 ftx");
    assert_eq!(
        sibling_container(&mbs).expect("解析").as_deref(),
        Some(ftx.as_path())
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// 数字后缀的变体容器：唯一候选取之，多个候选报错。
#[test]
fn falls_back_to_numeric_suffix_variants() {
    let dir = std::env::temp_dir().join("vw_atlas_suffix_test");
    std::fs::create_dir_all(&dir).expect("创建临时目录");
    let mbs = dir.join("Fighter_M.mbs");
    std::fs::write(&mbs, b"").expect("写 mbs");

    let variant = dir.join("Fighter_M00.ftx");
    std::fs::write(&variant, b"").expect("写变体");
    assert_eq!(
        sibling_container(&mbs).expect("解析").as_deref(),
        Some(variant.as_path())
    );

    std::fs::write(dir.join("Fighter_M10.ftx"), b"").expect("写第二个变体");
    assert!(sibling_container(&mbs).is_err(), "多个候选必须报错而不是猜");

    std::fs::remove_dir_all(&dir).ok();
}
