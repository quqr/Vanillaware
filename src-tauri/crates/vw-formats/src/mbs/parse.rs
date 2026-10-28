//! `.mbs` → [`V55`]：按分节表读取，再摊平成 PHP 原样的 JSON 模型。
//!
//! 三段职责各自独立，本文件只做**串联**：
//! - 分节的 `sp` / `count` / `stride` 来自 [`super::profile`]（抄自 PHP `$gp_data`）；
//! - 记录的字段布局来自 `spec/ksy/vw_mbs_v77.ksy`（= `vw-gen` 生成物）；
//! - 取值到 JSON 的映射规则（`'0x…'` 串、`fog_quad` 退化、s9 `name` 的截断……）
//!   逐条对照 PHP `FMBS_s*` 实现。
//!
//! 本解析器**不做**任何语义解释（不改矩阵、不查 blend 表）：那是 V55 → Quad 的事。

use std::path::Path;

use kaitai::{BytesReader, KStruct};
use vw_gen::gen::vw_mbs_v77::VwMbsV77;

use super::profile::{self, Profile};
use super::v55::{FogQuad, S3Rec, S4Rec, S5Rec, S6Rec, S7Rec, S8Rec, S9Rec, SaRec, SbRec, V55};
use crate::{Error, Result};

/// 从磁盘读一个 `.mbs` 并摊平成 [`V55`]。
pub fn parse_file(path: &Path, tag: &str) -> Result<V55> {
    let bytes = std::fs::read(path)?;
    parse_bytes(&bytes, tag)
}

/// 从内存字节解析；`tag` 决定分节表（见 [`profile::lookup`]）。
pub fn parse_bytes(bytes: &[u8], tag: &str) -> Result<V55> {
    let profile = lookup_tag(tag)?;
    // 大端家族（ps3_odin）需要 ksy 支持 `endian: expr`；本轮无语料，如实报错而非猜。
    if profile.big_endian {
        return Err(Error::Unsupported {
            what: "big-endian mbs",
            detail: format!("{tag} 的字节序解析尚未实现"),
        });
    }

    let counts = profile.read_counts(bytes)?;
    let io = BytesReader::from(bytes);
    let init = |root: &mut VwMbsV77| -> kaitai::KResult<()> {
        set_counts(root, &profile, counts);
        Ok(())
    };
    let root = <VwMbsV77 as KStruct>::read_into_with_init::<_, VwMbsV77>(&io, None, None, &init)?;

    let version = *root.version();
    if version != profile.version {
        return Err(Error::BadData(format!(
            "版本号 {version:#x} 与 tag {tag} 期望的 {:#x} 不符",
            profile.version
        )));
    }

    build(&root, &profile)
}

/// 标签 → 档案；未知标签报错而不是退回某个默认平台。
fn lookup_tag(tag: &str) -> Result<Profile> {
    profile::lookup(tag).ok_or_else(|| Error::Unsupported {
        what: "mbs tag",
        detail: tag.to_string(),
    })
}

/// 把 profile 的 `kind` 与 12 个 count 灌进生成物（ksy 用 params 而非 switch-on）。
fn set_counts(root: &mut VwMbsV77, profile: &Profile, counts: [u32; 12]) {
    root.set_params(
        profile.kind,
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        counts[4],
        counts[5],
        counts[6],
        counts[7],
        counts[8],
        counts[9],
        counts[10],
        counts[11],
    );
}

/// 逐分节摊平。每节的字段映射都与 PHP `FMBS_s*` 一一对应。
fn build(root: &VwMbsV77, profile: &Profile) -> Result<V55> {
    let s0 = root.s0_records()?;
    let s1 = root.s1_records()?;
    let s2 = root.s2_records()?;
    let s3 = root.s3_records()?;
    let s5 = root.s5_records()?;
    let s6 = root.s6_records()?;
    let s7 = root.s7_records()?;
    let s8 = root.s8_records()?;
    let s9 = root.s9_records()?;
    let sa = root.sa_records()?;
    let sb = root.sb_records()?;

    let out = V55 {
        s0: s0
            .iter()
            .map(|rec| fog_quad([&rec.c0()[..], &rec.c1()[..], &rec.c2()[..], &rec.c3()[..]]))
            .collect(),
        s1: s1.iter().map(|rec| quad8(&rec.floats()[..])).collect(),
        s2: s2.iter().map(|rec| quad8(&rec.floats()[..])).collect(),
        s3: s3
            .iter()
            .enumerate()
            .map(|(index, rec)| S3Rec {
                i: index_label("s3", index),
                rect: rec.rect().to_vec(),
                xyz: rec.xyz().to_vec(),
            })
            .collect(),
        s4: build_s4(root, profile)?,
        s5: s5
            .iter()
            .enumerate()
            .map(|(index, rec)| S5Rec {
                i: index_label("s5", index),
                s3: i64::from(*rec.s3()),
                bits: hex(*rec.flags()),
            })
            .collect(),
        s6: s6
            .iter()
            .enumerate()
            .map(|(index, rec)| S6Rec {
                i: index_label("s6", index),
                rect: vec![*rec.left(), *rec.top(), *rec.right(), *rec.bottom()],
                s4: vec![i64::from(*rec.s4_set()), i64::from(*rec.s4_no())],
                s5: vec![i64::from(*rec.s5_set()), i64::from(*rec.s5_no())],
                bits: hex(u32::from(*rec.flags())),
            })
            .collect(),
        s7: s7
            .iter()
            .enumerate()
            .map(|(index, rec)| S7Rec {
                i: index_label("s7", index),
                move_: rec.move_().to_vec(),
                rotate: rec.rotate().to_vec(),
                scale: rec.scale().to_vec(),
                fog: hash_hex(&rec.fog()[..]),
            })
            .collect(),
        s8: s8
            .iter()
            .enumerate()
            .map(|(index, rec)| S8Rec {
                i: index_label("s8", index),
                s6: i64::from(*rec.s6()),
                s7: i64::from(*rec.s7()),
                time: i64::from(*rec.frames()),
                loop_: i64::from(*rec.loop_()),
                sfx: i64::from(*rec.sfx()),
                bits: hex(*rec.flags()),
                in_s5s3: i64::from(*rec.in_s5s3()),
                in_s7: i64::from(*rec.in_s7()),
                in_s6: i64::from(*rec.in_s6()),
                in_s0s1s2: i64::from(*rec.in_s0s1s2()),
            })
            .collect(),
        s9: s9
            .iter()
            .enumerate()
            .map(|(index, rec)| S9Rec {
                i: index_label("s9", index),
                rect: vec![*rec.left(), *rec.top(), *rec.right(), *rec.bottom()],
                name: s9_name(rec),
                sa: vec![i64::from(*rec.sa_set()), i64::from(*rec.sa_no())],
            })
            .collect(),
        sa: sa
            .iter()
            .enumerate()
            .map(|(index, rec)| {
                // PHP：起点已按 `st` 上移，条数相应下移（见 `FMBS_sa`）。
                let set = i64::from(*rec.s8_set()) + i64::from(*rec.st());
                let no = i64::from(*rec.s8_no()) - i64::from(*rec.st());
                SaRec {
                    i: index_label("sa", index),
                    s8: vec![set, no, i64::from(*rec.s8_sum())],
                }
            })
            .collect(),
        sb: sb
            .iter()
            .enumerate()
            .map(|(index, _)| SbRec {
                i: index_label("sb", index),
            })
            .collect(),
        tag: profile.tag.to_string(),
        id3: profile.id3.to_string(),
        ver: "55".to_string(),
    };

    Ok(out)
}

/// s4 的两代记录定义字段名一致，仅 `attrib` 宽度（u2 / u4）不同，故分两路取。
fn build_s4(root: &VwMbsV77, profile: &Profile) -> Result<Vec<S4Rec>> {
    if profile.kind == 0 {
        let recs = root.s4_records_v72()?;
        Ok(recs
            .iter()
            .enumerate()
            .map(|(index, rec)| S4Rec {
                i: index_label("s4", index),
                blend: i64::from(*rec.blend()),
                tex: i64::from(*rec.tex()),
                s0s1s2: vec![
                    i64::from(*rec.s0()),
                    i64::from(*rec.s1()),
                    i64::from(*rec.s2()),
                ],
                bits: hex(u32::from(*rec.flags())),
                attr: hex(u32::from(*rec.attrib())),
                color: i64::from(*rec.color()),
            })
            .collect())
    } else {
        let recs = root.s4_records_v76()?;
        Ok(recs
            .iter()
            .enumerate()
            .map(|(index, rec)| S4Rec {
                i: index_label("s4", index),
                blend: i64::from(*rec.blend()),
                tex: i64::from(*rec.tex()),
                s0s1s2: vec![
                    i64::from(*rec.s0()),
                    i64::from(*rec.s1()),
                    i64::from(*rec.s2()),
                ],
                bits: hex(u32::from(*rec.flags())),
                attr: hex(*rec.attrib()),
                color: i64::from(*rec.color()),
            })
            .collect())
    }
}

/// 记录标识：PHP `sect_data_txt` 的 `"<节> <下标>"`。
fn index_label(section: &str, index: usize) -> String {
    format!("{section} {index}")
}

/// s1 / s2：PHP `nds_quad30p` 从 12 个 float 里取下标 2..9，共 8 个。
pub fn quad8(floats: &[f32]) -> Vec<f32> {
    floats.get(2..10).map(<[f32]>::to_vec).unwrap_or_default()
}

/// s0 的 `fog`：PHP `fog_quad` —— 四色全等退化为单串，否则四串。
pub fn fog_quad(colors: [&[u8]; 4]) -> FogQuad {
    let same = colors[0] == colors[1] && colors[0] == colors[2] && colors[0] == colors[3];
    if same {
        FogQuad::One(hash_hex(colors[0]))
    } else {
        FogQuad::Four([
            hash_hex(colors[0]),
            hash_hex(colors[1]),
            hash_hex(colors[2]),
            hash_hex(colors[3]),
        ])
    }
}

/// s7 的 `fog` 与 s0 的 `fog_quad` 同形：`'#' + 小写十六进制`。
fn hash_hex(bytes: &[u8]) -> String {
    format!("#{}", to_hex(bytes))
}

/// s9 的 `name`：PHP `substr0($s, 0x10)`。
///
/// `$s` 是整条 0x30 记录，`substr0` 从 0x10 起读到首个 NUL（或记录尾）。
/// 也就是说，若 0x10..0x27 这 24 字节里没有 NUL，读取会**越过** name 字段进入
/// 0x28 起的 `sa_set` 等字段。为了逐字节等价，这里补上那 8 字节尾部。
fn s9_name(rec: &vw_gen::gen::vw_mbs_v77::VwMbsV77_RecS9) -> String {
    let mut raw = rec.name().to_vec();
    raw.extend_from_slice(&rec.sa_set().to_le_bytes());
    raw.push(*rec.sa_no());
    raw.push(*rec.sa_main());
    raw.extend_from_slice(&rec.sasb_set().to_le_bytes());
    raw.push(*rec.sasb_no());
    raw.push(*rec.b2f());
    c_string(&raw)
}

/// 截断于首个 NUL；无 NUL 则取全长（PHP `substr0`）。
pub fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// PHP `'0x' . dechex($x)`：小写、无前导零。
fn hex(value: u32) -> String {
    format!("0x{value:x}")
}

/// 小写十六进制串（PHP `bin2hex`）。
fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
