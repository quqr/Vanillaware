//! 结构等价比对（判据见 ADR-0006）。
//!
//! 用途有两个，共用同一套递归比较：
//! - V55 ↔ PHP `.mbs.v55`：两侧都是本工具/权威 PHP 的原样 schema，期望**零差异**；
//! - Quad ↔ 旧 C# golden：先按 ADR-0006 的闭集差异表把 golden 规范化，再零差异比较。
//!
//! 判据本身不含任何「容忍」：键序与空白不参与比较，数值按 `1e-6` 容差，
//! 其余任何不同都记一条 [`Diff`]。**比对不通过就是不通过**，不允许在比较器里放宽。

use serde_json::Value;

/// 浮点比较容差（绝对下限）：PHP 把 f32 经 `number_format(x, 6)` 落盘，误差上界 5e-7。
pub const TOLERANCE: f64 = 1e-6;

/// 浮点比较的相对容差：旧 C# golden 按 `float` 的**最短往返表示**落盘（约 7 位有效数字），
/// 本工具把 f32 精确展宽成 f64，二者最多差半个 f32 ulp（≈ `|x| * 1.19e-7`）。
///
/// 只对「两侧都是 f32 来源」的数值有意义，但比较器无法区分来源，故统一按此放宽；
/// 它比 1 个 f32 ulp 还紧，不会掩盖真正的错位（见 ADR-0006 第 11 项）。
pub const RELATIVE_TOLERANCE: f64 = 2e-7;

/// 一处字段级差异。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    /// 字段路径，如 `s4[3].attr`。
    pub path: String,
    /// 期望值（真值侧）。
    pub expected: String,
    /// 实际值（本工具侧）。
    pub actual: String,
}

impl std::fmt::Display for Diff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: 期望 {} 实际 {}", self.path, self.expected, self.actual)
    }
}

/// 递归比较两个 JSON 值，差异全部追加到 `out`。
///
/// 规则：
/// - 对象：键集合必须相等（多键/缺键各记一条），然后逐键递归；
/// - 数组：长度必须相等（不等即记一条并停止比较该数组），然后逐元素递归；
/// - 数值：`|a - b| <= tolerance` 视为相等；
/// - 其余（字符串/布尔/null）：精确相等。
pub fn diff_values(expected: &Value, actual: &Value, path: &str, out: &mut Vec<Diff>) {
    diff_values_with(expected, actual, path, &tolerance_for, out);
}

/// 同 [`diff_values`]，但数值容差由调用方给（矩阵比对需要更宽的界，见 ADR-0006 第 9 项）。
pub fn diff_values_with(
    expected: &Value,
    actual: &Value,
    path: &str,
    tolerance: &dyn Fn(f64, f64) -> f64,
    out: &mut Vec<Diff>,
) {
    match (expected, actual) {
        (Value::Object(expected_map), Value::Object(actual_map)) => {
            for (key, expected_child) in expected_map {
                match actual_map.get(key) {
                    Some(actual_child) => {
                        diff_values_with(expected_child, actual_child, &child_path(path, key), tolerance, out);
                    }
                    None => out.push(Diff {
                        path: child_path(path, key),
                        expected: compact(expected_child),
                        actual: "<缺失>".to_string(),
                    }),
                }
            }
            for key in actual_map.keys() {
                if !expected_map.contains_key(key) {
                    out.push(Diff {
                        path: child_path(path, key),
                        expected: "<缺失>".to_string(),
                        actual: compact(&actual_map[key]),
                    });
                }
            }
        }
        (Value::Array(expected_items), Value::Array(actual_items)) => {
            if expected_items.len() != actual_items.len() {
                out.push(Diff {
                    path: path.to_string(),
                    expected: format!("长度 {}", expected_items.len()),
                    actual: format!("长度 {}", actual_items.len()),
                });
                return;
            }
            for (index, (expected_child, actual_child)) in
                expected_items.iter().zip(actual_items).enumerate()
            {
                diff_values_with(
                    expected_child,
                    actual_child,
                    &format!("{path}[{index}]"),
                    tolerance,
                    out,
                );
            }
        }
        (Value::Number(expected_number), Value::Number(actual_number)) => {
            let same = match (expected_number.as_f64(), actual_number.as_f64()) {
                (Some(a), Some(b)) => (a - b).abs() <= tolerance(a, b),
                _ => expected_number == actual_number,
            };
            if !same {
                out.push(Diff {
                    path: path.to_string(),
                    expected: expected_number.to_string(),
                    actual: actual_number.to_string(),
                });
            }
        }
        _ => {
            if expected != actual {
                out.push(Diff {
                    path: path.to_string(),
                    expected: compact(expected),
                    actual: compact(actual),
                });
            }
        }
    }
}

/// 单次比较允许的偏差：绝对下限 + f32 有效位相对项。
fn tolerance_for(a: f64, b: f64) -> f64 {
    TOLERANCE + RELATIVE_TOLERANCE * a.abs().max(b.abs())
}

/// 键路径拼接：顶层为 `$` 时直接返回键名，避免出现 `$.s0`。
fn child_path(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// 差异行里的值：单行紧凑 JSON，方便肉眼读。
fn compact(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<无法序列化>".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 数值在容差内即相等；键序无关。
    #[test]
    fn numbers_compare_within_tolerance() {
        let mut out = Vec::new();
        diff_values(&json!({"a": 0.274658, "b": 20000}), &json!({"b": 20000.0, "a": 0.2746582}), "", &mut out);
        assert!(out.is_empty(), "{out:?}");
    }

    /// 大数值按 f32 有效位放宽：C# 写 `2218.0312`，本工具写 `2218.03125`。
    #[test]
    fn large_numbers_use_relative_tolerance() {
        let mut out = Vec::new();
        diff_values(&json!([2218.0312, 1058.2188]), &json!([2218.03125, 1058.21875]), "", &mut out);
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
}
