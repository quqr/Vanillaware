use vw_formats::mbs::verify::*;
use serde_json::json;

/// 数值在容差内即相等；键序无关。
#[test]
fn numbers_compare_within_tolerance() {
    let mut out = Vec::new();
    diff_values(
        &json!({"a": 0.274658, "b": 20000}),
        &json!({"b": 20000.0, "a": 0.2746582}),
        "",
        &mut out,
    );
    assert!(out.is_empty(), "{out:?}");
}

/// 大数值按 f32 有效位放宽：C# 写 `2218.0312`，本工具写 `2218.03125`。
#[test]
fn large_numbers_use_relative_tolerance() {
    let mut out = Vec::new();
    diff_values(
        &json!([2218.0312, 1058.2188]),
        &json!([2218.03125, 1058.21875]),
        "",
        &mut out,
    );
    assert!(out.is_empty(), "{out:?}");

    // 真正的错位（远超 1 个 f32 ulp）仍要报出来。
    let mut out = Vec::new();
    diff_values(&json!([2218.0312]), &json!([2218.25]), "", &mut out);
    assert_eq!(out.len(), 1);
}

/// 超出容差、键缺失、数组长度不同都要报出来。
#[test]
fn mismatches_are_reported_with_paths() {
    let expected = json!({"s4": [{"attr": "0x1"}]});
    let actual = json!({"s4": [{"attr": "0x2"}]});
    let mut out = Vec::new();
    diff_values(&expected, &actual, "", &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].path, "s4[0].attr");

    let mut out = Vec::new();
    diff_values(&json!({"a": 1}), &json!({}), "", &mut out);
    assert_eq!(out[0].path, "a");
    assert_eq!(out[0].actual, "<缺失>");

    let mut out = Vec::new();
    diff_values(&json!({"a": [1, 2]}), &json!({"a": [1]}), "", &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].path, "a");
}
