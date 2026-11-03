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
