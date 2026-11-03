use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldPropDto {
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMapDto {
    pub name: String,
    pub group: String,
    pub files: Vec<String>,
    pub layers: u32,
    pub models: u32,
    pub props: Vec<FieldPropDto>,
    pub tiles: u32,
    pub window: [f32; 4],
}

/// The whole layer set as one strip, plus the world box it covers, so the
/// viewer can draw a minimap and turn a click into a world x.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldOverviewDto {
    pub image: Option<String>,
    pub span: [f32; 4],
}
