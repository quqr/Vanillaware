//! MBS 校验（判据见 ADR-0006）：旧 C# golden 的规范化 + 结构等价比对 + 自洽性检查。
//!
//! 两条路各司其职：
//! - **比对模式**：`.<name>.quad.json`（旧 C# 工具产物）先经 [`normalize_golden`] 规范化到 PHP 形态，
//!   再零差异比较。闭集外的任何差异**判失败**，不留后门。
//! - **自洽性检查**：无 golden 的平台（v76/v72）用它兜底，逐条核对结构不变量，
//!   返回的是「如实登记的能力边界」，不是「伪造的通过」。
//!
//! 规范化**只改形，不改值**：任何无法用形态对齐的差异都必须留在 [`Report::diffs`] 里失败。

use serde_json::{Map, Value};

use super::quad::{bitflag_attr, Attach, Pad, Quad};
use super::verify::{self, Diff};

/// 一份校验报告。
#[derive(Debug, Default)]
pub struct Report {
    /// 判失败的字段级差异（结构等价）。
    pub diffs: Vec<Diff>,
    /// 只报告、不判失败：矩阵平移列（下标 `3/7/11`）的差异——两侧是不同算法，
    /// 见 ADR-0006 第 1 项，这是能力边界而非已验证。
    pub translation_diffs: Vec<Diff>,
}

impl Report {
    /// 是否通过：没有任何判失败的差异。
    pub fn is_pass(&self) -> bool {
        self.diffs.is_empty()
    }
}

/// 与 golden（旧 C# 产物）比对。`ours` 是本工具的 Quad，`golden` 是 C# 的 `.quad.json`。
pub fn compare_golden(ours: &Quad, golden: &Value) -> crate::Result<Report> {
    let ours = serde_json::to_value(ours)
        .map_err(|e| crate::Error::BadData(format!("序列化 Quad 失败：{e}")))?;
    let mut report = Report::default();

    // 平移列的差异只登记，必须在「收缩成 3×3」之前采集。
    collect_translation_diffs(golden, &ours, &mut report.translation_diffs);

    let tag = ours.get("tag");
    let mut expected = normalize_golden(golden);
    // 顶层 `tag` 是 idtag 解码结果，与 Rust 侧同源，故从本工具侧补进去（不构成独立验证）。
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

    // 矩阵单独比：两侧的 3×3 块与平移列本来就不是同一算法（ADR-0006 第 1/9 项）。
    collect_matrix_diffs(&expected, &actual, &mut report.diffs);
    strip_matrix(&mut expected);
    strip_matrix(&mut actual);

    verify::diff_values(&expected, &actual, "", &mut report.diffs);
    Ok(report)
}

/// 把旧 C# 工具的 golden 规范化成 PHP 形态（ADR-0006 闭集差异表）。
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
    // C# 的挂载打包在 `{"attach":[…]}` 里，PHP 直接就是那对 `attach`，故取出内层。
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

/// PHP `quadfile::blend('normal')`——本轮 12 个档案的 tag 全落 default 分支。
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

/// 顶层稠密数组：C# 用 `null` 填空位，PHP `list_add` 补整数 `0`。
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

/// `keyframe` 一条：C# 与 PHP 同形。
fn norm_keyframe(item: &Value) -> Value {
    serde_json::json!({
        "debug": field(item, "debug"),
        "name": field(item, "name"),
        "layer": padded_list(item.get("layer"), norm_keyframe_layer),
    })
}

/// `keyframe` 的一个图层：只有可省略键的**出现条件**需要规范化。
fn norm_keyframe_layer(item: &Value) -> Value {
    let mut layer = Map::new();
    layer.insert("debug".to_string(), field(item, "debug"));
    layer.insert("dstquad".to_string(), field(item, "dstquad"));
    layer.insert("blend_id".to_string(), field(item, "blend_id"));

    // `fogquad`：PHP 仅在 s0 值 `!== '#ffffffff'` 时写入，C# 恒写（空时写 null）。
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
    // `attribute`：C# 存整数，PHP 存 `bitflag_attr(...)` 的串（单条为串、多条为数组）。
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

/// `hitbox` 一条：C# 与 PHP 同形。
fn norm_hitbox(item: &Value) -> Value {
    serde_json::json!({
        "name": field(item, "name"),
        "layer": padded_list(item.get("layer"), |layer| serde_json::json!({
            "debug": field(layer, "debug"),
            "hitquad": field(layer, "hitquad"),
        })),
    })
}

/// `animation` 一条：`loop_id` 恒有键，C#/PHP 都是整数。
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

/// 时间线一帧：`attach` / `color` / `matrix` 三键的缺省形态与 PHP 对齐。
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
    // `matrix`：C# 单位矩阵写 `null`，PHP 写空数组 `[]`；非单位时两边都是 16 个 f64。
    let matrix = match item.get("matrix") {
        Some(Value::Array(items)) => Value::Array(items.clone()),
        _ => Value::Array(Vec::new()),
    };
    entry.insert("matrix".to_string(), matrix);
    Value::Object(entry)
}

/// `skeleton` 一条：`{name, bone:[{attach}]}`，C# 与 PHP 同形。
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

/// 取键；缺席即 `null`（`diff_values` 会把它当作差异报出来）。
fn field(item: &Value, key: &str) -> Value {
    item.get(key).cloned().unwrap_or(Value::Null)
}

/// 矩阵数值容差：`1e-5 + 1e-5 * |矩阵块最大分量|`。
///
/// PHP 先把 s7 的 scale/rotate/move 经 `number_format(x, 6)` 收敛再算，旧 C# 用原始 f32；
/// 角度上 5e-7 的差经三角与连乘后，误差随**缩放**线性放大（实测到 1e-5 量级）。
/// 故相对项按整块的最大分量取，而不是按单个元素——否则「大缩放 × 近零元素」会被误判。
/// 这是 ADR-0006 第 9 项的能力边界，不是「矩阵已验证」。
const MATRIX_TOLERANCE: f64 = 1e-5;
const MATRIX_RELATIVE: f64 = 1e-5;

/// 取时间线一帧的 3×3 块（9 项）；空数组（单位矩阵）展开成单位矩阵本身。
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

/// 逐帧比较 3×3 块。缩放、旋转、平移列都不在这里（平移列见 [`collect_translation_diffs`]）。
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

/// 去掉两条时间线上的 `matrix` 键，让结构等价比较器不再管它。
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

/// 采集矩阵平移列（`3/7/11`）的差异，只登记不判失败。
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

/// 对两侧的 `keyframe[*].layer` 做「首色投影」（ADR-0006 第 12 项）。
///
/// 旧 C# 的 s0 解析只读第一个 4 字节色（`ParseNdsQuad18c` 取 `0x04` 处的 4 字节），
/// 而 PHP `fog_quad` 读 `0x04/0x08/0x0c/0x10` 四个色、四色全等才退化成单串。
/// 因此 golden 只能验证**首色**，以及「首色为 `#ffffffff` 时整键省略」这条规则；
/// 第 2~4 个色在 golden 里根本不存在——这是登记过的能力边界，不是验证通过。
///
/// 投影对两侧都施加，于是四色数组也归约成首色，比较器不会把「PHP 的数组 vs C# 的单串」
/// 当成数值差异。
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

/// 去掉「旧 C# 无法表达」的 `attribute`（ADR-0006 第 14 项）。
///
/// C# 用 `int.TryParse(hex, NumberStyles.HexNumber)` 解析 attrib，`>= 0x80000000` 的值
/// 超出 `int` 正数范围，解析失败后按 `0` 处理，于是整个 `attribute` 键被省略；
/// PHP `hexdec` 没有这个上限。本工具的 `bitflag_attr` 会把 bit31 渲染成 `keyattr_80000000`，
/// 故检出该串即说明这层差异——两侧一律删键，其余位仍逐项比较。
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

/// 去掉 `keyframe[*].layer` / `hitbox[*].layer` 的**尾部**空位（ADR-0006 第 13 项）。
///
/// PHP `quadfile::list_add` 先把整个 layer 补满 `0` 再填条目，故尾部空位一定保留；
/// 旧 C# 只在真正写入某条时才 `Add(null)`，因此尾部空位被丢掉。中间空位两边一致。
/// 尾部空位不承载任何信息，故对两侧都做同一处理后再比。
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

/// 空位：PHP/C# 都把它渲染成整数 `0`（C# 的 `null` 已在规范化时改成 `0`）。
fn is_pad(value: &Value) -> bool {
    value.as_u64() == Some(0)
}

/// 自洽性检查（无 golden 的平台用它兜底，ADR-0006）：核对转换结果的结构不变量。
///
/// 返回不变量违背清单；空列表表示全部成立。**不包含任何「以为对」的判断**：
/// 每条都能指向一个具体下标。
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
                    "{path}.dstquad 有 {} 个分量，应为 8",
                    layer.dstquad.len()
                ));
            }
            if let Some(src) = &layer.srcquad {
                if src.len() != 8 {
                    issues.push(format!("{path}.srcquad 有 {} 个分量，应为 8", src.len()));
                }
            }
            // `blend_id` 指向平台的 blend 表，而 Quad 只物化 `normal` 一项（PHP 亦然），
            // 故不能拿它跟 `quad.blend.len()` 比；只检查非负。
            if layer.blend_id < 0 {
                issues.push(format!("{path}.blend_id 为负：{}", layer.blend_id));
            }
            if layer.tex_id.is_some_and(|tex| tex < 0) {
                issues.push(format!("{path}.tex_id 为负"));
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
                    "hitbox[{index}].layer[{layer_index}].hitquad 有 {} 个分量，应为 8",
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
                    "{path} 应为 {} id={index}，实际 {} id={}",
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
                "animation[{index}].loop_id = {} 超出时间线 {}",
                animation.loop_id,
                animation.timeline.len()
            ));
        }
        for (time_index, entry) in animation.timeline.iter().enumerate() {
            let path = format!("animation[{index}].timeline[{time_index}]");
            if !matches!(entry.matrix.len(), 0 | 16) {
                issues.push(format!(
                    "{path}.matrix 有 {} 项，应为 0 或 16",
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

/// `name` 必须与 PHP `sprintf('keyframe %d', …)` 的产物一致。
fn expect_name(issues: &mut Vec<String>, name: &str, expected: &str, section: &str) {
    if name != expected {
        issues.push(format!("{section}.name = {name}，应为 {expected}"));
    }
}

/// 挂载点必须落在对应数组的**实填**位上。
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
            "{path}.attach 指向 {}[{}]：不存在",
            attach.kind, attach.id
        ));
    }
}

/// PHP `! empty($list[$key])`：只有「在位且实填」才算非空。
fn filled<T>(cell: &Pad<T>) -> Option<&T> {
    match cell {
        Pad::Filled(value) => Some(value),
        Pad::Empty(_) => None,
    }
}
