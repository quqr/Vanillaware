use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

use super::v55::{FogQuad, S7Rec, S8Rec, V55};
use super::{idtag, matrix};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Pad<T> {
    Empty(u8),
    Filled(T),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Attach {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Blend {
    pub name: String,
    pub mode_rgb: [String; 3],
    pub mode_alpha: [String; 3],
    pub debug: [String; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MixItem {
    Index(u8),
    Mode(String),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Attribute {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KeyframeLayer {
    pub debug: (String, i64, i64, String),
    pub dstquad: Vec<f32>,
    pub blend_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fogquad: Option<FogQuad>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tex_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srcquad: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<Attribute>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colorize: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Keyframe {
    pub debug: String,
    pub name: String,
    pub layer: Vec<Pad<KeyframeLayer>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HitboxLayer {
    pub debug: String,
    pub hitquad: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hitbox {
    pub name: String,
    pub layer: Vec<Pad<HitboxLayer>>,
}

/// Raw `s7` fields, which is what the engine interpolates (`f_700058c0a0` lerps
/// the eight floats and the four fog bytes, then rebuilds the rotation matrix
/// from the mixed angles). Kept out of the serialised form so the PHP oracle
/// comparison stays byte-identical.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transform {
    pub move_: [f32; 3],
    pub rotate: [f32; 3],
    pub scale: [f32; 2],
}

impl Transform {
    pub fn from_s7(s7: &S7Rec) -> Self {
        let at = |values: &[f32], index: usize| values.get(index).copied().unwrap_or(0.0);
        Self {
            move_: [at(&s7.move_, 0), at(&s7.move_, 1), at(&s7.move_, 2)],
            rotate: [at(&s7.rotate, 0), at(&s7.rotate, 1), at(&s7.rotate, 2)],
            scale: [at(&s7.scale, 0), at(&s7.scale, 1)],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimelineEntry {
    pub debug: String,
    pub time: i64,
    pub matrix_mix_id: i64,
    pub color_mix_id: i64,
    pub dstquad_mix_id: i64,
    pub fogquad_mix_id: i64,
    pub hitquad_mix_id: i64,
    /// Interpolation step count (`in_rate`), kept out of the serialised form so
    /// the PHP oracle comparison stays byte-identical.
    #[serde(default, skip)]
    pub in_rate: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<Attach>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub matrix: Vec<f64>,
    #[serde(skip)]
    pub transform: Transform,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Animation {
    pub name: String,
    pub timeline: Vec<TimelineEntry>,
    pub loop_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bone {
    pub attach: Attach,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Skeleton {
    pub name: String,
    pub bone: Vec<Bone>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Quad {
    pub tag: Value,
    pub blend: Vec<Blend>,
    pub mix: [MixItem; 3],
    pub keyframe: Vec<Pad<Keyframe>>,
    pub hitbox: Vec<Pad<Hitbox>>,
    pub slot: Vec<Pad<[Attach; 2]>>,
    pub animation: Vec<Pad<Animation>>,
    pub skeleton: Vec<Pad<Skeleton>>,
}

struct SaAnim {
    time: Vec<S8Rec>,
    loop_id: i64,
}

pub fn convert(v55: &V55) -> Result<Quad> {
    let game = game_name(&v55.id3);
    let tag = idtag::lookup(game)
        .and_then(|decoded| decoded.get("tag").cloned())
        .ok_or_else(|| Error::Unsupported {
            what: "idtag",
            detail: format!("no idtag matches <{game}> (or the idtag failed to decode)"),
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

pub fn game_name(id3: &str) -> &str {
    match id3.find(' ') {
        Some(at) => &id3[at + 1..],
        None => id3.get(1..).unwrap_or(""),
    }
}

fn vanilla_blendmode(tag: &str) -> Result<Vec<Blend>> {
    match tag {
        "ps2_grim" | "ps2_odin" => Err(Error::Unsupported {
            what: "ps2 blend",
            detail: format!("the ps2 blend of {tag} raises TypeError in PHP strict_types"),
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

        if has_keyframe && has_hitbox {
            let id = s6k as i64;
            set_at(
                &mut quad.slot,
                s6k,
                Pad::Filled([attach("keyframe", id), attach("hitbox", id)]),
            );
        }

        let _ = flags;
    }
    Ok(())
}

fn sas8_loop(v55: &V55) -> Vec<Pad<SaAnim>> {
    let mut salist: Vec<Pad<SaAnim>> = Vec::new();

    for (sak, sav) in v55.sa.iter().enumerate() {
        let base = field(&sav.s8, 0);
        let mut step: i64 = 0;
        let mut time: Vec<S8Rec> = Vec::new();
        let mut loop_id: i64 = -1;
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

fn s8_flags_last(flags: u64, tag: &str) -> bool {
    if tag == "ps2_odin" {
        flags & 0x08 != 0
    } else {
        flags & 0x800 != 0
    }
}

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
                    in_rate: s8v.rate,
                    attach,
                    color: None,
                    matrix,
                    transform: Transform::from_s7(s7v),
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

fn attach(kind: &str, id: i64) -> Attach {
    Attach {
        kind: kind.to_string(),
        id,
    }
}

pub fn set_at<T>(list: &mut Vec<Pad<T>>, key: usize, value: Pad<T>) {
    while list.len() <= key {
        list.push(Pad::Empty(0));
    }
    list[key] = value;
}

fn pad_vec<T>(len: usize) -> Vec<Pad<T>> {
    let mut out = Vec::with_capacity(len);
    out.resize_with(len, || Pad::Empty(0));
    out
}

fn filled_at<T>(list: &[Pad<T>], key: usize) -> Option<&T> {
    match list.get(key) {
        Some(Pad::Filled(value)) => Some(value),
        _ => None,
    }
}

fn field(list: &[i64], index: usize) -> i64 {
    list.get(index).copied().unwrap_or(0)
}

fn out_of_range(section: &str, index: i64) -> Error {
    Error::BadData(format!("{section}[{index}] out of range"))
}

fn s4_at(v55: &V55, index: i64) -> Result<&super::v55::S4Rec> {
    if index < 0 {
        return Err(out_of_range("s4", index));
    }
    v55.s4
        .get(index as usize)
        .ok_or_else(|| out_of_range("s4", index))
}

pub fn hexdec(text: &str) -> u64 {
    let mut value = 0u64;
    for ch in text.chars() {
        if let Some(digit) = ch.to_digit(16) {
            value = value * 16 + u64::from(digit);
        }
    }
    value
}

pub fn bitflag_attr(value: u64, prefix: &str) -> Attribute {
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
