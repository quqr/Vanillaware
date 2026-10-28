use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrackDto {
    pub index: u32,
    pub name: String,
    pub cue_id: i32,
    /// HCA / Opus / ADX …
    pub codec: String,
    pub playable: bool,
    pub stream: bool,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBundleDto {
    /// "acb" / "awb"
    pub container: String,
    pub source: String,
    pub tracks: Vec<AudioTrackDto>,
    pub codecs: Vec<AudioCodecCountDto>,
    pub external: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCodecCountDto {
    pub codec: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioClipDto {
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub frames: u64,
    pub duration_ms: u64,
    pub stride: u32,
    pub skip: u32,
    pub bytes: u64,
}
