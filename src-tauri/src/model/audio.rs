//! 音频容器与片段的跨进程结构。

use serde::Serialize;

/// 一条轨道（cue 或 AFS2 条目）。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrackDto {
    /// 容器内顺序号，取片段时按它定位。
    pub index: u32,
    pub name: String,
    pub cue_id: i32,
    /// HCA / Opus / ADX …
    pub codec: String,
    /// 该编码能否在应用内解码播放。
    pub playable: bool,
    /// 载荷在外部 AWB 里（而不是内嵌）。
    pub stream: bool,
    /// 载荷字节数（AWB 已知，ACB 为 0）。
    pub bytes: u64,
}

/// 容器索引。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBundleDto {
    /// "acb" / "awb"
    pub container: String,
    pub source: String,
    pub tracks: Vec<AudioTrackDto>,
    /// 编码 → 条数，按条数降序。
    pub codecs: Vec<AudioCodecCountDto>,
    /// ACB 引用的同目录 AWB。
    pub external: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCodecCountDto {
    pub codec: String,
    pub count: u32,
}

/// 片段元信息；二进制载荷另走 `audio_clip_data`。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioClipDto {
    /// "hca"（后端已解成 i16 PCM）或 "lopus"（裸 Opus 包）。
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub frames: u64,
    pub duration_ms: u64,
    /// lopus 每包字节数；HCA 为 0。
    pub stride: u32,
    /// 需要丢弃的前导样本。
    pub skip: u32,
    /// `audio_clip_data` 会返回的字节数。
    pub bytes: u64,
}
