use serde_json::Value;

pub const TOLERANCE: f64 = 1e-6;

pub const RELATIVE_TOLERANCE: f64 = 2e-7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub path: String,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for Diff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: expected {}, got {}",
            self.path, self.expected, self.actual
        )
    }
}

pub fn diff_values(expected: &Value, actual: &Value, path: &str, out: &mut Vec<Diff>) {
    diff_values_with(expected, actual, path, &tolerance_for, out);
}

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
                        diff_values_with(
                            expected_child,
                            actual_child,
                            &child_path(path, key),
                            tolerance,
                            out,
                        );
                    }
                    None => out.push(Diff {
                        path: child_path(path, key),
                        expected: compact(expected_child),
                        actual: "<missing>".to_string(),
                    }),
                }
            }
            for key in actual_map.keys() {
                if !expected_map.contains_key(key) {
                    out.push(Diff {
                        path: child_path(path, key),
                        expected: "<missing>".to_string(),
                        actual: compact(&actual_map[key]),
                    });
                }
            }
        }
        (Value::Array(expected_items), Value::Array(actual_items)) => {
            if expected_items.len() != actual_items.len() {
                out.push(Diff {
                    path: path.to_string(),
                    expected: format!("length {}", expected_items.len()),
                    actual: format!("length {}", actual_items.len()),
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

fn tolerance_for(a: f64, b: f64) -> f64 {
    TOLERANCE + RELATIVE_TOLERANCE * a.abs().max(b.abs())
}

fn child_path(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

fn compact(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<not serialisable>".to_string())
}
