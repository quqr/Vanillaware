use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialoguePoolDto {
    pub path: Option<String>,
    pub strings: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueStatsDto {
    pub entries: u32,
    pub commands: u32,
    pub text_commands: u32,
    pub unique_ids: u32,
    pub missing_text: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueLineDto {
    pub at: u32,
    pub opcode: u32,
    pub text_id: Option<u32>,
    pub speaker: Option<String>,
    pub voice: Option<u32>,
    pub text: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueEntryDto {
    pub index: u32,
    pub name: String,
    pub group: i16,
    pub link: i32,
    pub kind: u32,
    pub offset: u32,
    pub commands: u32,
    pub lines: Vec<DialogueLineDto>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueDto {
    pub entries: Vec<DialogueEntryDto>,
    pub stats: DialogueStatsDto,
    pub pool: DialoguePoolDto,
    pub warnings: Vec<String>,
}
