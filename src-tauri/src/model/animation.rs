use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertDto {
    pub mbs_path: String,
    pub tag: String,
    pub quad: serde_json::Value,
    pub self_check: Vec<String>,
}
