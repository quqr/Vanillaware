use std::path::Path;

use kaitai::{BytesReader, KStruct};
use schema::gen::mbs_v77::MbsV77;

use super::profile::{self, Profile};
use super::v55::{FogQuad, S3Rec, S4Rec, S5Rec, S6Rec, S7Rec, S8Rec, S9Rec, SaRec, SbRec, V55};
use crate::{Error, Result};

pub fn parse_file(path: &Path, tag: &str) -> Result<V55> {
    let bytes = std::fs::read(path)?;
    parse_bytes(&bytes, tag)
}

pub fn parse_bytes(bytes: &[u8], tag: &str) -> Result<V55> {
    let profile = lookup_tag(tag)?;
    if profile.big_endian {
        return Err(Error::Unsupported {
            what: "big-endian mbs",
            detail: format!("byte order parsing for {tag} is not implemented yet"),
        });
    }

    let counts = profile.read_counts(bytes)?;
    let io = BytesReader::from(bytes);
    let init = |root: &mut MbsV77| -> kaitai::KResult<()> {
        set_counts(root, &profile, counts);
        Ok(())
    };
    let root = <MbsV77 as KStruct>::read_into_with_init::<_, MbsV77>(&io, None, None, &init)?;

    let version = *root.version();
    if version != profile.version {
        return Err(Error::BadData(format!(
            "version {version:#x} does not match the 0x{:#x} expected for tag {tag}",
            profile.version
        )));
    }

    build(&root, &profile)
}

fn lookup_tag(tag: &str) -> Result<Profile> {
    profile::lookup(tag).ok_or_else(|| Error::Unsupported {
        what: "mbs tag",
        detail: tag.to_string(),
    })
}

fn set_counts(root: &mut MbsV77, profile: &Profile, counts: [u32; 12]) {
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

fn build(root: &MbsV77, profile: &Profile) -> Result<V55> {
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

fn build_s4(root: &MbsV77, profile: &Profile) -> Result<Vec<S4Rec>> {
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

fn index_label(section: &str, index: usize) -> String {
    format!("{section} {index}")
}

pub fn quad8(floats: &[f32]) -> Vec<f32> {
    floats.get(2..10).map(<[f32]>::to_vec).unwrap_or_default()
}

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

fn hash_hex(bytes: &[u8]) -> String {
    format!("#{}", to_hex(bytes))
}

fn s9_name(rec: &schema::gen::mbs_v77::MbsV77_RecS9) -> String {
    let mut raw = rec.name().to_vec();
    raw.extend_from_slice(&rec.sa_set().to_le_bytes());
    raw.push(*rec.sa_no());
    raw.push(*rec.sa_main());
    raw.extend_from_slice(&rec.sasb_set().to_le_bytes());
    raw.push(*rec.sasb_no());
    raw.push(*rec.b2f());
    c_string(&raw)
}

pub fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn hex(value: u32) -> String {
    format!("0x{value:x}")
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
