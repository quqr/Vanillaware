use std::path::PathBuf;

use tauri::State;

use crate::model::dialogue::{
    DialogueDto, DialogueEntryDto, DialogueLineDto, DialoguePoolDto, DialogueStatsDto,
};
use crate::state::AppState;

#[tauri::command]
pub async fn dialogue_open(
    state: State<'_, AppState>,
    path: String,
) -> Result<DialogueDto, String> {
    let target = PathBuf::from(&path);
    let service = state.dialogue.clone();

    tokio::task::spawn_blocking(move || {
        let dialogue = service.open(&target)?;
        Ok(DialogueDto {
            entries: dialogue
                .entries
                .into_iter()
                .map(|entry| DialogueEntryDto {
                    index: entry.index,
                    name: entry.name,
                    group: entry.group,
                    link: entry.link,
                    kind: entry.kind,
                    offset: entry.offset,
                    commands: entry.commands,
                    lines: entry
                        .lines
                        .into_iter()
                        .map(|line| DialogueLineDto {
                            at: line.at,
                            opcode: line.opcode,
                            text_id: (line.text_id != u32::MAX).then_some(line.text_id),
                            speaker: line.speaker,
                            voice: line.voice,
                            text: line.text,
                        })
                        .collect(),
                })
                .collect(),
            stats: DialogueStatsDto {
                entries: dialogue.stats.entries,
                commands: dialogue.stats.commands,
                text_commands: dialogue.stats.text_commands,
                unique_ids: dialogue.stats.unique_ids,
                missing_text: dialogue.stats.missing_text,
            },
            pool: DialoguePoolDto {
                path: dialogue
                    .pool_path
                    .map(|value| value.to_string_lossy().into_owned()),
                strings: dialogue.pool_count,
            },
            warnings: dialogue.warnings,
        })
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}
