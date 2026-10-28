//! USM 影片索引的跨进程结构。

use serde::Serialize;

/// 影片索引。帧表只给「大小 + 关键帧」，字节按 GOP 另取。
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
    /// SPS / PPS 原始字节（hex），前端据此构造 avcC。
    pub sps: String,
    pub pps: String,
    /// 音轨编码：adx / hca / none。
    pub audio: String,
    pub chunks: u32,
    pub stream_bytes: u64,
    /// 关键帧的帧号。
    pub keyframes: Vec<u32>,
    /// 每帧字节数，用于码率时间轴。
    pub frame_sizes: Vec<u32>,
}
