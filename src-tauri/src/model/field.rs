use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMapDto {
    pub name: String,
    pub group: String,
    pub files: Vec<String>,
    pub layers: u32,
    pub tiles: u32,
    pub window: [f32; 4],
}
