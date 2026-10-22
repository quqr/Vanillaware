//! 纹理帧相关的 DTO。

use serde::Serialize;

/// 单帧元信息（不含像素）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameMetaDto {
    pub index: u32,
    pub width: u32,
    pub height: u32,
    pub format: u16,
    pub format_name: &'static str,
    pub channels: u8,
}

/// 一个纹理容器的帧清单。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameListDto {
    pub path: String,
    pub count: u32,
    pub frames: Vec<FrameMetaDto>,
}
