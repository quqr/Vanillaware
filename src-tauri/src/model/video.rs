use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsmVideoDto {
    pub width: u32,
    pub height: u32,
    /// `avc1.640028`
    pub codec: String,
    pub profile: u32,
    pub level: u32,
    pub fps: f32,
    pub frames: u32,
    pub duration_ms: u64,
    pub sps: String,
    pub pps: String,
    pub audio: String,
    pub chunks: u32,
    pub stream_bytes: u64,
    pub keyframes: Vec<u32>,
    pub frame_sizes: Vec<u32>,
}
