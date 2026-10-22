//! V55 → Quad：逐行移植 PHP `rip-vanillaware/v55_to_quad.php`（ADR-0004：真值是 PHP）。
//!
//! 三段职责：
//! - [`convert`] 串起 `vanilla()` 的调用序（idtag → blend → mix → s6 → sa → s9）；
//! - [`s6_loop`] 出 `keyframe` / `hitbox` / `slot`；
//! - [`sas8_loop`] 把 `sa` 的 s8 序列走成时间线（含 jump/last 与循环检测），
//!   再由 [`q3d_sas8s9_loop`] 摊成 `animation` / `skeleton`。
//!
//! 有几处 PHP 行为是**刻意**保留的，别当 bug 改掉：
//! - `list_add` 用**整数 0** 填空位（[`Pad::Empty`]），所以数组是稠密的；
//! - `matrix` 键**恒存在**（PHP 拿数组与 `0` 做 `!==` 恒为真），单位矩阵时是 `[]`；
//! - `attribute` / `colorize` / `fogquad` / `tex_id` / `srcquad` 仅在条件命中时才出现，
//!   其余键一律缺省；
//! - 矩阵运算的输入是 `.v55` 里**已被 `number_format(x, 6)` 收敛过**的十进制值，
//!   故 [`.matrix`](super::matrix) 先做 `round6` 再算，否则缩放/旋转的乘积会把
//!   f32 的舍入误差放大到 `1e-6` 容差之外（见 ADR-0006）。

use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

use super::v55::{FogQuad, S8Rec, V55};
use super::{idtag, matrix};
use crate::{Error, Result};

/// `quadfile::list_add` 的空位：PHP 用整数 `0` 填充，故存的是「占位数字」。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Pad<T> {
    /// 空位，恒为 `0`。
    Empty(u8),
    /// 实填条目。
    Filled(T),
}

/// `quadfile::attach` 的 `{ "type" : …, "id" : … }`。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Attach {
    /// `"slot"` / `"keyframe"` / `"hitbox"` / `"animation"`。
    #[serde(rename = "type")]
    pub kind: String,
    /// 被引用条目的下标。
    pub id: i64,
}

/// `quadfile::blend('normal')` 的产物。
///
/// 本轮 12 个档案（[`super::profile::lookup`]）里的 tag 全部落在 PHP `vanilla_blendmode`
/// 的 default 分支，故 `debug` 恒为 2 条字符串。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Blend {
    pub name: String,
    pub mode_rgb: [String; 3],
    pub mode_alpha: [String; 3],
    pub debug: [String; 2],
}

/// `vanilla_mixing` 的一项：下标整数或混合模式名。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MixItem {
    /// `0`：直取。
    Index(u8),
    /// `"LINEAR"` / `"CATMULL ROM"`。
    Mode(String),
}

/// `bitflag_attr` 的产物：单条返回字符串，多条返回数组（`int < 1` 的整数 `0`
/// 分支不可达，因为调用点已判 `attr > 0`）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Attribute {
    One(String),
    Many(Vec<String>),
}

/// 判定框（`keyframe`）里的一个图层。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KeyframeLayer {
    /// `[flags, blend, color, attr]`——PHP 直接塞进一个混合数组。
    pub debug: (String, i64, i64, String),
    /// s2：目标四边形，8 个 float。
    pub dstquad: Vec<f32>,
    /// s4 的 blend 表下标。
    pub blend_id: i64,
    /// s0 雾色；等于 `#ffffffff` 时整键省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fogquad: Option<FogQuad>,
    /// s4 的图集下标；仅 `s4_flags('tex')` 命中时出现。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tex_id: Option<i64>,
    /// s1 源四边形；与 `tex_id` 同进同出。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srcquad: Option<Vec<f32>>,
    /// `bitflag_attr(attr, 'keyattr')`；`attr > 0` 时才出现。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<Attribute>,
    /// `"COLOR_%x"`；`color > 0` 时才出现。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colorize: Option<String>,
}

/// `keyframe` 一节的一条：一个 s6 帧的全部图层。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Keyframe {
    /// s6 的 flags 串。
    pub debug: String,
    /// `"keyframe %d"`。
    pub name: String,
    /// 稠密图层数组，被 `s4_flags('skip')` 跳过的位置是 `0`。
    pub layer: Vec<Pad<KeyframeLayer>>,
}

/// 命中框里的一个图层。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HitboxLayer {
    /// s5 的 flags 串。
    pub debug: String,
    /// s3 的 `rect`，8 个 float。
    pub hitquad: Vec<f32>,
}

/// `hitbox` 一节的一条。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hitbox {
    /// `"hitbox %d"`。
    pub name: String,
    pub layer: Vec<Pad<HitboxLayer>>,
}

/// `animation` 时间线上的一帧。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimelineEntry {
    /// s8 的 flags 串。
    pub debug: String,
    /// s8 的时长（帧）。
    pub time: i64,
    /// 对应 s8 的 `in_s7`（矩阵与颜色共用一个混合下标）。
    pub matrix_mix_id: i64,
    pub color_mix_id: i64,
    /// 对应 s8 的 `in_s6`（目标/雾色共用）。
    pub dstquad_mix_id: i64,
    pub fogquad_mix_id: i64,
    /// 对应 s8 的 `in_s5s3`。
    pub hitquad_mix_id: i64,
    /// 命中 slot / keyframe / hitbox 时的挂载点。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<Attach>,
    /// s7 的雾色；等于 `#ffffffff` 时整键省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// 16 个 f64（行主序）；单位矩阵时为空数组，但**键恒存在**。
    pub matrix: Vec<f64>,
}

/// `animation` 一节的一条：一根骨的整条时间线。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Animation {
    /// `"animation %d"`。
    pub name: String,
    pub timeline: Vec<TimelineEntry>,
    /// `sa` 的循环起点（时间线内顺序下标）；无环为 `-1`。
    ///
    /// PHP 侧该项恒有键：`sa` 缺席时 `$salist[$sak]['loop']` 取到 `null`，
    /// 而 `jsonpretty` 把 `null` 渲染成整数 `0`，故这里也用 `i64` 而不是 `Option`。
    pub loop_id: i64,
}

/// `skeleton` 里的一根骨。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bone {
    pub attach: Attach,
}

/// `skeleton` 一节的一条：一个 s9 的骨列表。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Skeleton {
    /// s9 的 `name`。
    pub name: String,
    pub bone: Vec<Bone>,
}

/// 一个 `.mbs` 转换后的完整 Quad 视图（键序与 PHP 插入序一致）。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Quad {
    /// idtag 解码结果（见 [`idtag`]）。
    pub tag: Value,
    pub blend: Vec<Blend>,
    pub mix: [MixItem; 3],
    pub keyframe: Vec<Pad<Keyframe>>,
    pub hitbox: Vec<Pad<Hitbox>>,
    /// 每条 `[keyframe, hitbox]` 的挂载对。
    pub slot: Vec<Pad<[Attach; 2]>>,
    pub animation: Vec<Pad<Animation>>,
    pub skeleton: Vec<Pad<Skeleton>>,
}

/// `sa` 一节走完 s8 序列后的结果（[`sas8_loop`] 的内部产物）。
struct SaAnim {
    /// 走过的 s8 记录，顺序即时间线顺序。
    time: Vec<S8Rec>,
    /// 循环起点（`time` 内的顺序下标）；无环为 `-1`。
    loop_id: i64,
}

/// 对应 PHP `vanilla`：`.v55` → Quad。
pub fn convert(v55: &V55) -> Result<Quad> {
    let game = game_name(&v55.id3);
    // PHP：`$quad = quadfile::load_idtag($game)`，而 `load_idtag` 返回的是
    // `idtag_decode` 的结果 `{"tag": {…}}`；随后 `$quad['tag']` 取的正是里层字典。
    // 故顶层 `tag` 存的是**解码后的字典本身**，不是那层包装。
    let tag = idtag::lookup(game)
        .and_then(|decoded| decoded.get("tag").cloned())
        .ok_or_else(|| Error::Unsupported {
            what: "idtag",
            detail: format!("没有匹配 <{game}> 的 idtag（或 idtag 解码失败）"),
        })?;

    let mut quad = Quad {
        tag,
        blend: vanilla_blendmode(&v55.tag)?,
        mix: vanilla_mixing(&v55.tag),
        keyframe: Vec::new(),
        hitbox: Vec::new(),
        slot: Vec::new(),
        animation: Vec::new(),
        skeleton: Vec::new(),
    };

    s6_loop(v55, &mut quad)?;
    let salist = sas8_loop(v55);
    q3d_sas8s9_loop(v55, &salist, &mut quad)?;
    Ok(quad)
}

/// PHP：`substr($id3, strpos($id3, ' ') + 1)`——取 `id3` 里第一个空格之后的部分。
///
/// `strpos` 未命中时是 `false`，`false + 1` 为 `1`，故退化成「从第 1 个字节起」。
fn game_name(id3: &str) -> &str {
    match id3.find(' ') {
        Some(at) => &id3[at + 1..],
        None => id3.get(1..).unwrap_or(""),
    }
}

/// 对应 PHP `vanilla_blendmode`。
///
/// `ps2_grim` / `ps2_odin` 走 `ps2::blend`，而 PHP 的调用点以整数 `0` 传
/// `string $a`，在 `strict_types=1` 下必然抛 `TypeError`——本工具如实报不支持，
/// 不替它编一份「本该崩溃才有的」输出（ADR-0004 能力边界）。
fn vanilla_blendmode(tag: &str) -> Result<Vec<Blend>> {
    match tag {
        "ps2_grim" | "ps2_odin" => Err(Error::Unsupported {
            what: "ps2 blend",
            detail: format!("{tag} 的 ps2 blend 在 PHP 里因 strict_types 抛 TypeError"),
        }),
        _ => Ok(vec![Blend {
            name: "normal".to_string(),
            mode_rgb: [
                "FUNC_ADD".to_string(),
                "SRC_ALPHA".to_string(),
                "ONE_MINUS_SRC_ALPHA".to_string(),
            ],
            mode_alpha: [
                "FUNC_ADD".to_string(),
                "ONE".to_string(),
                "ONE_MINUS_SRC_ALPHA".to_string(),
            ],
            debug: [
                "SRC.rgb * SRC.a + DST.rgb * (1 - SRC.a)".to_string(),
                "SRC.a           + DST.a   * (1 - SRC.a)".to_string(),
            ],
        }]),
    }
}

/// 对应 PHP `vanilla_mixing`。
fn vanilla_mixing(tag: &str) -> [MixItem; 3] {
    let linear = || MixItem::Mode("LINEAR".to_string());
    match tag {
        "ps2_grim" | "ps2_odin" => [
            MixItem::Index(0),
            linear(),
            MixItem::Mode("CATMULL ROM".to_string()),
        ],
        "nds_kuma" => [MixItem::Index(0), MixItem::Index(0), MixItem::Index(0)],
        _ => [MixItem::Index(0), linear(), linear()],
    }
}

/// 对应 PHP `s6_loop`：出 `keyframe` / `hitbox` / `slot` 三节。
fn s6_loop(v55: &V55, quad: &mut Quad) -> Result<()> {
    for (s6k, s6v) in v55.s6.iter().enumerate() {
        let flags = hexdec(&s6v.bits);

        // section keyframe
        let s4_base = field(&s6v.s4, 0);
        let s4_count = field(&s6v.s4, 1);
        let mut has_keyframe = false;
        if s4_count > 0 {
            let mut layer: Vec<Pad<KeyframeLayer>> = pad_vec(s4_count as usize);
            for (i, cell) in layer.iter_mut().enumerate() {
                let s4v = s4_at(v55, s4_base + i as i64)?;
                let flags4 = hexdec(&s4v.bits);
                if flags4 & 0x02 != 0 {
                    continue; // s4_flags('skip')
                }

                let s2v = v55
                    .s2
                    .get(field(&s4v.s0s1s2, 2) as usize)
                    .ok_or_else(|| out_of_range("s2", field(&s4v.s0s1s2, 2)))?
                    .clone();

                let mut data = KeyframeLayer {
                    debug: (s4v.bits.clone(), s4v.blend, s4v.color, s4v.attr.clone()),
                    dstquad: s2v,
                    blend_id: s4v.blend,
                    fogquad: None,
                    tex_id: None,
                    srcquad: None,
                    attribute: None,
                    colorize: None,
                };

                let s0k = field(&s4v.s0s1s2, 0);
                let s0v = v55
                    .s0
                    .get(s0k as usize)
                    .ok_or_else(|| out_of_range("s0", s0k))?;
                if !matches!(s0v, FogQuad::One(color) if color == "#ffffffff") {
                    data.fogquad = Some(s0v.clone());
                }

                if flags4 & 0x04 == 0 {
                    // s4_flags('tex')
                    data.tex_id = Some(s4v.tex);
                    let s1k = field(&s4v.s0s1s2, 1);
                    data.srcquad = Some(
                        v55.s1
                            .get(s1k as usize)
                            .ok_or_else(|| out_of_range("s1", s1k))?
                            .clone(),
                    );
                }

                let attr = hexdec(&s4v.attr);
                if attr > 0 {
                    data.attribute = Some(bitflag_attr(attr, "keyattr"));
                }
                if s4v.color > 0 {
                    data.colorize = Some(format!("COLOR_{:x}", s4v.color as u64));
                }

                *cell = Pad::Filled(data);
            }

            set_at(
                &mut quad.keyframe,
                s6k,
                Pad::Filled(Keyframe {
                    debug: s6v.bits.clone(),
                    name: format!("keyframe {s6k}"),
                    layer,
                }),
            );
            has_keyframe = true;
        }

        // section hitbox
        let s5_base = field(&s6v.s5, 0);
        let s5_count = field(&s6v.s5, 1);
        let mut has_hitbox = false;
        if s5_count > 0 {
            let mut layer: Vec<Pad<HitboxLayer>> = pad_vec(s5_count as usize);
            for (i, cell) in layer.iter_mut().enumerate() {
                let s5v = v55
                    .s5
                    .get((s5_base + i as i64) as usize)
                    .ok_or_else(|| out_of_range("s5", s5_base + i as i64))?;
                let s3v = v55
                    .s3
                    .get(s5v.s3 as usize)
                    .ok_or_else(|| out_of_range("s3", s5v.s3))?;
                *cell = Pad::Filled(HitboxLayer {
                    debug: s5v.bits.clone(),
                    hitquad: s3v.rect.clone(),
                });
            }

            set_at(
                &mut quad.hitbox,
                s6k,
                Pad::Filled(Hitbox {
                    name: format!("hitbox {s6k}"),
                    layer,
                }),
            );
            has_hitbox = true;
        }

        // section slot：只有 keyframe 与 hitbox 同时在场才建挂载对。
        if has_keyframe && has_hitbox {
            let id = s6k as i64;
            set_at(
                &mut quad.slot,
                s6k,
                Pad::Filled([attach("keyframe", id), attach("hitbox", id)]),
            );
        }

        // PHP 只把 s6 的 flags 存进全局给 `s6_flags` 用，该函数恒返回 false，故无需落地。
        let _ = flags;
    }
    Ok(())
}

/// 对应 PHP `sas8_loop`：按 `sa` 的起点沿 s8 链走，走回已访问项即认为是环。
fn sas8_loop(v55: &V55) -> Vec<Pad<SaAnim>> {
    let mut salist: Vec<Pad<SaAnim>> = Vec::new();

    for (sak, sav) in v55.sa.iter().enumerate() {
        let base = field(&sav.s8, 0);
        let mut step: i64 = 0;
        let mut time: Vec<S8Rec> = Vec::new();
        let mut loop_id: i64 = -1;
        // s8 下标 → 它在 `time` 里的顺序下标；命中即为环。
        let mut line: HashMap<usize, usize> = HashMap::new();

        loop {
            let key = base + step;
            let Some(s8v) = (key >= 0).then(|| v55.s8.get(key as usize)).flatten() else {
                break; // ! isset($gp_json['s8'][$s8k])
            };
            let flags = hexdec(&s8v.bits);

            if let Some(&position) = line.get(&(key as usize)) {
                loop_id = position as i64;
                break;
            }
            line.insert(key as usize, time.len());
            time.push(s8v.clone());

            if flags & 0x04 != 0 {
                // s8_flags('jump')：last 与 jump 同时命中时跳转优先。
                step = s8v.loop_;
            } else if s8_flags_last(flags, &v55.tag) {
                break;
            } else {
                step += 1;
            }
        }

        set_at(&mut salist, sak, Pad::Filled(SaAnim { time, loop_id }));
    }

    salist
}

/// 对应 PHP `s8_flags('last')`：仅 `ps2_odin` 用 `0x08`，其余平台用 `0x800`。
fn s8_flags_last(flags: u64, tag: &str) -> bool {
    if tag == "ps2_odin" {
        flags & 0x08 != 0
    } else {
        flags & 0x800 != 0
    }
}

/// 对应 PHP `q3D_sas8s9_loop`：把 `salist` 沿 `s9` 摊成 `animation` / `skeleton`。
fn q3d_sas8s9_loop(v55: &V55, salist: &[Pad<SaAnim>], quad: &mut Quad) -> Result<()> {
    for (s9k, s9v) in v55.s9.iter().enumerate() {
        let sa_count = field(&s9v.sa, 1);
        if sa_count < 1 {
            continue;
        }

        let sa_base = field(&s9v.sa, 0);
        let mut bone: Vec<Bone> = Vec::new();
        for i in 0..sa_count as usize {
            let sak = sa_base + i as i64;
            // `$salist[$sak]` 缺席 / 是整数 0 时，PHP 对非数组 `foreach` 直接跳过，
            // 并把 `loop_id` 取成 `null`（`jsonpretty` 再渲染成 `0`）。
            let (time, loop_id) = match salist.get(sak as usize) {
                Some(Pad::Filled(sa)) => (sa.time.as_slice(), sa.loop_id),
                _ => (&[][..], 0),
            };

            let mut timeline = Vec::with_capacity(time.len());
            for s8v in time {
                let flags = hexdec(&s8v.bits);

                let s6k = s8v.s6 as usize;
                let attach = if filled_at(&quad.slot, s6k).is_some() {
                    Some(attach("slot", s8v.s6))
                } else if filled_at(&quad.keyframe, s6k).is_some() {
                    Some(attach("keyframe", s8v.s6))
                } else if filled_at(&quad.hitbox, s6k).is_some() {
                    Some(attach("hitbox", s8v.s6))
                } else {
                    None
                };

                let s7v = v55
                    .s7
                    .get(s8v.s7 as usize)
                    .ok_or_else(|| out_of_range("s7", s8v.s7))?;
                let matrix = matrix::s7_matrix(
                    &s7v.move_,
                    &s7v.rotate,
                    &s7v.scale,
                    flags & 0x01 != 0,
                    flags & 0x02 != 0,
                );

                let mut entry = TimelineEntry {
                    debug: s8v.bits.clone(),
                    time: s8v.time,
                    matrix_mix_id: s8v.in_s7,
                    color_mix_id: s8v.in_s7,
                    dstquad_mix_id: s8v.in_s6,
                    fogquad_mix_id: s8v.in_s6,
                    hitquad_mix_id: s8v.in_s5s3,
                    attach,
                    color: None,
                    matrix,
                };
                if s7v.fog != "#ffffffff" {
                    entry.color = Some(s7v.fog.clone());
                }
                timeline.push(entry);
            }

            set_at(
                &mut quad.animation,
                sak as usize,
                Pad::Filled(Animation {
                    name: format!("animation {sak}"),
                    timeline,
                    loop_id,
                }),
            );
            bone.push(Bone {
                attach: attach("animation", sak),
            });
        }

        set_at(
            &mut quad.skeleton,
            s9k,
            Pad::Filled(Skeleton {
                name: s9v.name.clone(),
                bone,
            }),
        );
    }
    Ok(())
}

/// 对应 PHP `quadfile::attach`。
fn attach(kind: &str, id: i64) -> Attach {
    Attach {
        kind: kind.to_string(),
        id,
    }
}

/// 对应 PHP `quadfile::list_add`：不足处补整数 `0`，然后按下标落位。
///
/// 因为每条 s6 / sa / s9 都是按下标升序访问，落位后数组恒为稠密的 `0..=k`。
fn set_at<T>(list: &mut Vec<Pad<T>>, key: usize, value: Pad<T>) {
    while list.len() <= key {
        list.push(Pad::Empty(0));
    }
    list[key] = value;
}

/// 一张全为 `0` 空位的定长表格（`list_add` 的补位结果）。
fn pad_vec<T>(len: usize) -> Vec<Pad<T>> {
    let mut out = Vec::with_capacity(len);
    out.resize_with(len, || Pad::Empty(0));
    out
}

/// PHP `! empty($list[$key])`：只有「在位且实填」才算非空。
fn filled_at<T>(list: &[Pad<T>], key: usize) -> Option<&T> {
    match list.get(key) {
        Some(Pad::Filled(value)) => Some(value),
        _ => None,
    }
}

/// 取定长整形数组的第 `index` 项；PHP 越界取到 `null`，参与运算时即 `0`。
fn field(list: &[i64], index: usize) -> i64 {
    list.get(index).copied().unwrap_or(0)
}

/// 越界报错：PHP 在这类下标上会直接 fatal，本工具如实报错而不是猜一个值。
fn out_of_range(section: &str, index: i64) -> Error {
    Error::BadData(format!("{section}[{index}] 越界"))
}

/// s4 的下标取值（越界即报错）。
fn s4_at(v55: &V55, index: i64) -> Result<&super::v55::S4Rec> {
    if index < 0 {
        return Err(out_of_range("s4", index));
    }
    v55.s4
        .get(index as usize)
        .ok_or_else(|| out_of_range("s4", index))
}

/// 对应 PHP `hexdec`：忽略所有非十六进制字符后按位累加（`"0x1e"` → `0x1e`）。
fn hexdec(text: &str) -> u64 {
    let mut value = 0u64;
    for ch in text.chars() {
        if let Some(digit) = ch.to_digit(16) {
            value = value * 16 + u64::from(digit);
        }
    }
    value
}

/// 对应 PHP `bitflag_attr($int, $prefix)`（`$int > 0` 已在调用点保证）。
///
/// 校验器要把旧 C# golden 的 `attribute` 整数规范化成同名串数组，故对 crate 内可见。
pub(crate) fn bitflag_attr(value: u64, prefix: &str) -> Attribute {
    let mut items = Vec::new();
    let mut rest = value;
    let mut shift = 0u32;
    while rest > 0 {
        if rest & 1 == 1 {
            items.push(format!("{prefix}_{:x}", 1u64 << shift));
        }
        rest >>= 1;
        shift += 1;
    }
    if items.len() == 1 {
        Attribute::One(items.remove(0))
    } else {
        Attribute::Many(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexdec_ignores_non_hex() {
        assert_eq!(hexdec("0x0"), 0);
        assert_eq!(hexdec("0x5"), 5);
        assert_eq!(hexdec("0x1e"), 0x1e);
        assert_eq!(hexdec("0x820"), 0x820);
    }

    #[test]
    fn bitflags_turn_into_names() {
        assert_eq!(
            bitflag_attr(0x5, "keyattr"),
            Attribute::Many(vec!["keyattr_1".to_string(), "keyattr_4".to_string(),])
        );
        assert_eq!(
            bitflag_attr(0x2, "keyattr"),
            Attribute::One("keyattr_2".to_string())
        );
    }

    #[test]
    fn game_name_drops_console_prefix() {
        assert_eq!(game_name("swit unicorn overlord"), "unicorn overlord");
        assert_eq!(game_name("ps4 13 sentinels"), "13 sentinels");
    }

    #[test]
    fn list_add_pads_with_zero() {
        let mut list: Vec<Pad<u8>> = Vec::new();
        set_at(&mut list, 2, Pad::Filled(7u8));
        assert_eq!(list, vec![Pad::Empty(0), Pad::Empty(0), Pad::Filled(7u8)]);
    }
}
