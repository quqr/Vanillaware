use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FogQuad {
    One(String),
    Four([String; 4]),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S3Rec {
    pub i: String,
    pub rect: Vec<f32>,
    pub xyz: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S4Rec {
    pub i: String,
    pub blend: i64,
    pub tex: i64,
    pub s0s1s2: Vec<i64>,
    pub bits: String,
    pub attr: String,
    pub color: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S5Rec {
    pub i: String,
    pub s3: i64,
    pub bits: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S6Rec {
    pub i: String,
    pub rect: Vec<f32>,
    pub s4: Vec<i64>,
    pub s5: Vec<i64>,
    pub bits: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S7Rec {
    pub i: String,
    #[serde(rename = "move")]
    pub move_: Vec<f32>,
    pub rotate: Vec<f32>,
    pub scale: Vec<f32>,
    pub fog: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S8Rec {
    pub i: String,
    pub s6: i64,
    pub s7: i64,
    pub time: i64,
    #[serde(rename = "loop")]
    pub loop_: i64,
    pub sfx: i64,
    pub bits: String,
    pub in_s5s3: i64,
    pub in_s7: i64,
    pub in_s6: i64,
    pub in_s0s1s2: i64,
    /// Interpolation step count kept out of the serialised form so the
    /// PHP oracle comparison stays byte-identical.
    #[serde(default, skip)]
    pub rate: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S9Rec {
    pub i: String,
    pub rect: Vec<f32>,
    pub name: String,
    pub sa: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaRec {
    pub i: String,
    pub s8: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SbRec {
    pub i: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V55 {
    pub s0: Vec<FogQuad>,
    pub s1: Vec<Vec<f32>>,
    pub s2: Vec<Vec<f32>>,
    pub s3: Vec<S3Rec>,
    pub s4: Vec<S4Rec>,
    pub s5: Vec<S5Rec>,
    pub s6: Vec<S6Rec>,
    pub s7: Vec<S7Rec>,
    pub s8: Vec<S8Rec>,
    pub s9: Vec<S9Rec>,
    pub sa: Vec<SaRec>,
    pub sb: Vec<SbRec>,
    pub tag: String,
    pub id3: String,
    pub ver: String,
}
