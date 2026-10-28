use serde_json::{Map, Value};

use super::quad::{bitflag_attr, Attach, Pad, Quad};
use super::verify::{self, Diff};

#[derive(Debug, Default)]
pub struct Report {
    pub diffs: Vec<Diff>,
    pub translation_diffs: Vec<Diff>,
}

impl Report {
    pub fn is_pass(&self) -> bool {
        self.diffs.is_empty()
    }
}

pub fn compare_golden(ours: &Quad, golden: &Value) -> crate::Result<Report> {
    let ours = serde_json::to_value(ours)
        .map_err(|e| crate::Error::BadData(format!("failed to serialise Quad: {e}")))?;
    let mut report = Report::default();

    collect_translation_diffs(golden, &ours, &mut report.translation_diffs);

    let tag = ours.get("tag");
    let mut expected = normalize_golden(golden);
    if let Some(tag) = tag {
        expected["tag"] = tag.clone();
    }
    let mut actual = ours;

    project_fogquad(&mut expected);
    project_fogquad(&mut actual);
    project_attribute(&mut expected);
    project_attribute(&mut actual);
    trim_layer_pads(&mut expected);
    trim_layer_pads(&mut actual);

    collect_matrix_diffs(&expected, &actual, &mut report.diffs);
    strip_matrix(&mut expected);
    strip_matrix(&mut actual);

    verify::diff_values(&expected, &actual, "", &mut report.diffs);
    Ok(report)
}

pub fn normalize_golden(golden: &Value) -> Value {
    let mut out = Map::new();
    out.insert("blend".to_string(), php_blend());
    out.insert(
        "mix".to_string(),
        serde_json::json!([0, "LINEAR", "LINEAR"]),
    );
    out.insert(
        "keyframe".to_string(),
        padded_list(golden.get("keyframe"), norm_keyframe),
    );
    out.insert(
        "hitbox".to_string(),
        padded_list(golden.get("hitbox"), norm_hitbox),
    );
    out.insert(
        "slot".to_string(),
        padded_list(golden.get("slot"), |item| field(item, "attach")),
    );
    out.insert(
        "animation".to_string(),
        padded_list(golden.get("animation"), norm_animation),
    );
    out.insert(
        "skeleton".to_string(),
        padded_list(golden.get("skeleton"), norm_skeleton),
    );
    Value::Object(out)
}

fn php_blend() -> Value {
    serde_json::json!([{
        "name": "normal",
        "mode_rgb": ["FUNC_ADD", "SRC_ALPHA", "ONE_MINUS_SRC_ALPHA"],
        "mode_alpha": ["FUNC_ADD", "ONE", "ONE_MINUS_SRC_ALPHA"],
        "debug": [
            "SRC.rgb * SRC.a + DST.rgb * (1 - SRC.a)",
            "SRC.a           + DST.a   * (1 - SRC.a)",
        ],
    }])
}

fn padded_list(value: Option<&Value>, shape: fn(&Value) -> Value) -> Value {
    let empty = Vec::new();
    let items = value.and_then(Value::as_array).unwrap_or(&empty);
    Value::Array(
        items
            .iter()
            .map(|item| {
                if item.is_null() {
                    Value::from(0)
                } else {
                    shape(item)
                }
            })
            .collect(),
    )
}

fn norm_keyframe(item: &Value) -> Value {
    serde_json::json!({
        "debug": field(item, "debug"),
        "name": field(item, "name"),
        "layer": padded_list(item.get("layer"), norm_keyframe_layer),
    })
}

fn norm_keyframe_layer(item: &Value) -> Value {
    let mut layer = Map::new();
    layer.insert("debug".to_string(), field(item, "debug"));
    layer.insert("dstquad".to_string(), field(item, "dstquad"));
    layer.insert("blend_id".to_string(), field(item, "blend_id"));

    if let Some(fog) = item.get("fogquad") {
        if !fog.is_null() && fog.as_str() != Some("#ffffffff") {
            layer.insert("fogquad".to_string(), fog.clone());
        }
    }
    for key in ["tex_id", "srcquad"] {
        if let Some(value) = item.get(key) {
            if !value.is_null() {
                layer.insert(key.to_string(), value.clone());
            }
        }
    }
    if let Some(number) = item.get("attribute").and_then(Value::as_u64) {
        if number > 0 {
            let attribute = bitflag_attr(number, "keyattr");
            layer.insert(
                "attribute".to_string(),
                serde_json::to_value(attribute).unwrap_or(Value::Null),
            );
        }
    }
    if let Some(value) = item.get("colorize") {
        if !value.is_null() {
            layer.insert("colorize".to_string(), value.clone());
        }
    }
    Value::Object(layer)
}

fn norm_hitbox(item: &Value) -> Value {
    serde_json::json!({
        "name": field(item, "name"),
        "layer": padded_list(item.get("layer"), |layer| serde_json::json!({
            "debug": field(layer, "debug"),
            "hitquad": field(layer, "hitquad"),
        })),
    })
}

fn norm_animation(item: &Value) -> Value {
    let empty = Vec::new();
    let timeline = item
        .get("timeline")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    serde_json::json!({
        "name": field(item, "name"),
        "timeline": timeline.iter().map(norm_timeline).collect::<Vec<_>>(),
        "loop_id": field(item, "loop_id"),
    })
}

fn norm_timeline(item: &Value) -> Value {
    let mut entry = Map::new();
    for key in [
        "debug",
        "time",
        "matrix_mix_id",
        "color_mix_id",
        "dstquad_mix_id",
        "fogquad_mix_id",
        "hitquad_mix_id",
    ] {
        entry.insert(key.to_string(), field(item, key));
    }
    for key in ["attach", "color"] {
        if let Some(value) = item.get(key) {
            if !value.is_null() {
                entry.insert(key.to_string(), value.clone());
            }
        }
    }
    let matrix = match item.get("matrix") {
        Some(Value::Array(items)) => Value::Array(items.clone()),
        _ => Value::Array(Vec::new()),
    };
    entry.insert("matrix".to_string(), matrix);
    Value::Object(entry)
}

fn norm_skeleton(item: &Value) -> Value {
    let empty = Vec::new();
    let bone = item.get("bone").and_then(Value::as_array).unwrap_or(&empty);
    serde_json::json!({
        "name": field(item, "name"),
        "bone": bone
            .iter()
            .map(|bone_item| serde_json::json!({"attach": field(bone_item, "attach")}))
            .collect::<Vec<_>>(),
    })
}

fn field(item: &Value, key: &str) -> Value {
    item.get(key).cloned().unwrap_or(Value::Null)
}

const MATRIX_TOLERANCE: f64 = 1e-5;
const MATRIX_RELATIVE: f64 = 1e-5;

fn matrix_block(entry: &Value) -> [f64; 9] {
    const IDENTITY: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let Some(items) = entry.get("matrix").and_then(Value::as_array) else {
        return IDENTITY;
    };
    if items.len() != 16 {
        return IDENTITY;
    }
    let mut block = [0f64; 9];
    for (slot, &source) in block.iter_mut().zip(&[0usize, 1, 2, 4, 5, 6, 8, 9, 10]) {
        *slot = items[source].as_f64().unwrap_or(0.0);
    }
    block
}

fn collect_matrix_diffs(expected: &Value, actual: &Value, out: &mut Vec<Diff>) {
    let empty = Vec::new();
    let expected_anims = expected
        .get("animation")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let actual_anims = actual
        .get("animation")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    for (anim_index, (expected_anim, actual_anim)) in
        expected_anims.iter().zip(actual_anims).enumerate()
    {
        let expected_tl = expected_anim
            .get("timeline")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        let actual_tl = actual_anim
            .get("timeline")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        for (time_index, (expected_entry, actual_entry)) in
            expected_tl.iter().zip(actual_tl).enumerate()
        {
            let expected_block = matrix_block(expected_entry);
            let actual_block = matrix_block(actual_entry);
            let magnitude = expected_block
                .iter()
                .chain(actual_block.iter())
                .fold(1.0f64, |bound, value| bound.max(value.abs()));
            let tolerance = MATRIX_TOLERANCE + MATRIX_RELATIVE * magnitude;
            for (index, (expected_value, actual_value)) in
                expected_block.iter().zip(actual_block.iter()).enumerate()
            {
                if (expected_value - actual_value).abs() > tolerance {
                    out.push(Diff {
                        path: format!(
                            "animation[{anim_index}].timeline[{time_index}].matrix[{index}]"
                        ),
                        expected: expected_value.to_string(),
                        actual: actual_value.to_string(),
                    });
                }
            }
        }
    }
}

fn strip_matrix(value: &mut Value) {
    let Some(anims) = value.get_mut("animation").and_then(Value::as_array_mut) else {
        return;
    };
    for anim in anims.iter_mut() {
        let Some(timeline) = anim.get_mut("timeline").and_then(Value::as_array_mut) else {
            continue;
        };
        for entry in timeline.iter_mut() {
            if let Some(object) = entry.as_object_mut() {
                object.remove("matrix");
            }
        }
    }
}

fn collect_translation_diffs(golden: &Value, ours: &Value, out: &mut Vec<Diff>) {
    let empty = Vec::new();
    let expected_anims = golden
        .get("animation")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let actual_anims = ours
        .get("animation")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    for (anim_index, (expected_anim, actual_anim)) in
        expected_anims.iter().zip(actual_anims).enumerate()
    {
        let expected_tl = expected_anim
            .get("timeline")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        let actual_tl = actual_anim
            .get("timeline")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        for (time_index, (expected_entry, actual_entry)) in
            expected_tl.iter().zip(actual_tl).enumerate()
        {
            for index in [3usize, 7, 11] {
                let path =
                    format!("animation[{anim_index}].timeline[{time_index}].matrix[{index}]");
                let (Some(expected), Some(actual)) = (
                    expected_entry
                        .get("matrix")
                        .and_then(|m| m.get(index))
                        .and_then(Value::as_f64),
                    actual_entry
                        .get("matrix")
                        .and_then(|m| m.get(index))
                        .and_then(Value::as_f64),
                ) else {
                    continue;
                };
                if (expected - actual).abs() > verify::TOLERANCE {
                    out.push(Diff {
                        path,
                        expected: expected.to_string(),
                        actual: actual.to_string(),
                    });
                }
            }
        }
    }
}

fn project_fogquad(value: &mut Value) {
    let Some(layers) = value
        .get_mut("keyframe")
        .and_then(Value::as_array_mut)
        .map(|keyframes| keyframes.iter_mut())
    else {
        return;
    };
    for keyframe in layers {
        let Some(layer_items) = keyframe.get_mut("layer").and_then(Value::as_array_mut) else {
            continue;
        };
        for layer in layer_items.iter_mut() {
            let Some(object) = layer.as_object_mut() else {
                continue;
            };
            let first = match object.get("fogquad") {
                Some(Value::String(color)) => color.clone(),
                Some(Value::Array(colors)) => colors
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                _ => continue,
            };
            if first == "#ffffffff" {
                object.remove("fogquad");
            } else {
                object.insert("fogquad".to_string(), Value::String(first));
            }
        }
    }
}

fn project_attribute(value: &mut Value) {
    let Some(keyframes) = value.get_mut("keyframe").and_then(Value::as_array_mut) else {
        return;
    };
    for keyframe in keyframes.iter_mut() {
        let Some(layer_items) = keyframe.get_mut("layer").and_then(Value::as_array_mut) else {
            continue;
        };
        for layer in layer_items.iter_mut() {
            let Some(object) = layer.as_object_mut() else {
                continue;
            };
            let overflows = match object.get("attribute") {
                Some(Value::String(name)) => name == "keyattr_80000000",
                Some(Value::Array(names)) => names
                    .iter()
                    .any(|name| name.as_str() == Some("keyattr_80000000")),
                _ => false,
            };
            if overflows {
                object.remove("attribute");
            }
        }
    }
}

fn trim_layer_pads(value: &mut Value) {
    for section in ["keyframe", "hitbox"] {
        let Some(entries) = value.get_mut(section).and_then(Value::as_array_mut) else {
            continue;
        };
        for entry in entries.iter_mut() {
            let Some(layer_items) = entry.get_mut("layer").and_then(Value::as_array_mut) else {
                continue;
            };
            while layer_items.last().is_some_and(is_pad) {
                layer_items.pop();
            }
        }
    }
}

fn is_pad(value: &Value) -> bool {
    value.as_u64() == Some(0)
}

pub fn self_check(quad: &Quad) -> Vec<String> {
    let mut issues = Vec::new();

    for (index, cell) in quad.keyframe.iter().enumerate() {
        let Some(keyframe) = filled(cell) else {
            continue;
        };
        expect_name(
            &mut issues,
            &keyframe.name,
            &format!("keyframe {index}"),
            "keyframe",
        );
        for (layer_index, cell) in keyframe.layer.iter().enumerate() {
            let Some(layer) = filled(cell) else { continue };
            let path = format!("keyframe[{index}].layer[{layer_index}]");
            if layer.dstquad.len() != 8 {
                issues.push(format!(
                    "{path}.dstquad has {} components, expected 8",
                    layer.dstquad.len()
                ));
            }
            if let Some(src) = &layer.srcquad {
                if src.len() != 8 {
                    issues.push(format!("{path}.srcquad has {} components, expected 8", src.len()));
                }
            }
            if layer.blend_id < 0 {
                issues.push(format!("{path}.blend_id is negative: {}", layer.blend_id));
            }
            if layer.tex_id.is_some_and(|tex| tex < 0) {
                issues.push(format!("{path}.tex_id is negative"));
            }
        }
    }

    for (index, cell) in quad.hitbox.iter().enumerate() {
        let Some(hitbox) = filled(cell) else { continue };
        expect_name(
            &mut issues,
            &hitbox.name,
            &format!("hitbox {index}"),
            "hitbox",
        );
        for (layer_index, cell) in hitbox.layer.iter().enumerate() {
            let Some(layer) = filled(cell) else { continue };
            if layer.hitquad.len() != 8 {
                issues.push(format!(
                    "hitbox[{index}].layer[{layer_index}].hitquad has {} components, expected 8",
                    layer.hitquad.len()
                ));
            }
        }
    }

    for (index, cell) in quad.slot.iter().enumerate() {
        let Some(pair) = filled(cell) else { continue };
        let kinds = ["keyframe", "hitbox"];
        for (slot_index, attach) in pair.iter().enumerate() {
            let path = format!("slot[{index}][{slot_index}]");
            if attach.kind != kinds[slot_index] || attach.id != index as i64 {
                issues.push(format!(
                    "{path} should be {} id={index}, got {} id={}",
                    kinds[slot_index], attach.kind, attach.id
                ));
            }
        }
    }

    for (index, cell) in quad.animation.iter().enumerate() {
        let Some(animation) = filled(cell) else {
            continue;
        };
        expect_name(
            &mut issues,
            &animation.name,
            &format!("animation {index}"),
            "animation",
        );
        if !animation.timeline.is_empty()
            && animation.loop_id >= 0
            && animation.loop_id as usize >= animation.timeline.len()
        {
            issues.push(format!(
                "animation[{index}].loop_id = {} exceeds the timeline {}",
                animation.loop_id,
                animation.timeline.len()
            ));
        }
        for (time_index, entry) in animation.timeline.iter().enumerate() {
            let path = format!("animation[{index}].timeline[{time_index}]");
            if !matches!(entry.matrix.len(), 0 | 16) {
                issues.push(format!(
                    "{path}.matrix has {} entries, expected 0 or 16",
                    entry.matrix.len()
                ));
            }
            if let Some(attach) = &entry.attach {
                check_attach(&mut issues, quad, attach, &path);
            }
        }
    }

    for (index, cell) in quad.skeleton.iter().enumerate() {
        let Some(skeleton) = filled(cell) else {
            continue;
        };
        for (bone_index, bone) in skeleton.bone.iter().enumerate() {
            let path = format!("skeleton[{index}].bone[{bone_index}]");
            if bone.attach.kind != "animation" {
                issues.push(format!("{path}.attach.kind = {}", bone.attach.kind));
            }
            check_attach(&mut issues, quad, &bone.attach, &path);
        }
    }

    issues
}

fn expect_name(issues: &mut Vec<String>, name: &str, expected: &str, section: &str) {
    if name != expected {
        issues.push(format!("{section}.name = {name}, expected {expected}"));
    }
}

fn check_attach(issues: &mut Vec<String>, quad: &Quad, attach: &Attach, path: &str) {
    let filled_at = match attach.kind.as_str() {
        "slot" => quad
            .slot
            .get(attach.id.max(0) as usize)
            .is_some_and(|cell| filled(cell).is_some()),
        "keyframe" => quad
            .keyframe
            .get(attach.id.max(0) as usize)
            .is_some_and(|cell| filled(cell).is_some()),
        "hitbox" => quad
            .hitbox
            .get(attach.id.max(0) as usize)
            .is_some_and(|cell| filled(cell).is_some()),
        "animation" => quad
            .animation
            .get(attach.id.max(0) as usize)
            .is_some_and(|cell| filled(cell).is_some()),
        _ => false,
    };
    if attach.id < 0 || !filled_at {
        issues.push(format!(
            "{path}.attach points at {}[{}]: missing",
            attach.kind, attach.id
        ));
    }
}

fn filled<T>(cell: &Pad<T>) -> Option<&T> {
    match cell {
        Pad::Filled(value) => Some(value),
        Pad::Empty(_) => None,
    }
}
