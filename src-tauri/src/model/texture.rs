use serde::Serialize;

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameListDto {
    pub path: String,
    pub count: u32,
    pub frames: Vec<FrameMetaDto>,
}
