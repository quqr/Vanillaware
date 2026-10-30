use kaitai::{BytesReader, KStruct};
use schema::gen::rsc::Rsc;
use schema::gen::rsc_command::RscCommand;

use super::resolve::TextPool;
use crate::Result;

const NO_VALUE: u32 = u32::MAX;

const TEXT_OPCODES: [u32; 2] = [0x189, 0x192];

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub entries: u32,
    pub commands: u32,
    pub text_commands: u32,
    pub unique_ids: u32,
    pub missing_text: u32,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub at: u32,
    pub opcode: u32,
    pub text_id: u32,
    pub speaker: Option<String>,
    pub voice: Option<u32>,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub index: u32,
    pub name: String,
    pub group: i16,
    pub link: i32,
    pub kind: u32,
    pub offset: u32,
    pub commands: u32,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone)]
pub struct Dialogue {
    pub entries: Vec<Entry>,
    pub stats: Stats,
    pub pool_path: Option<std::path::PathBuf>,
    pub pool_count: usize,
    pub warnings: Vec<String>,
}

pub fn parse(bytes: &[u8], pool: Option<&TextPool>) -> Result<Dialogue> {
    let io = BytesReader::from(bytes);
    let root = <Rsc as KStruct>::read_into::<_, Rsc>(&io, None, None)?;

    let handles = root.entries();
    let offsets: Vec<u32> = handles.iter().map(|entry| *entry.offset()).collect();

    let mut entries = Vec::with_capacity(handles.len());
    let mut warnings = Vec::new();
    let mut stats = Stats {
        entries: handles.len() as u32,
        ..Stats::default()
    };
    let mut ids = std::collections::BTreeSet::new();

    for (position, handle) in handles.iter().enumerate() {
        let offset = offsets[position];
        let start = offset as usize;
        let mut record = Entry {
            index: *handle.index(),
            name: field_name(&handle.name()),
            group: *handle.group(),
            link: *handle.link(),
            kind: *handle.kind(),
            offset,
            commands: 0,
            lines: Vec::new(),
        };

        if offset == 0 {
            entries.push(record);
            continue;
        }
        if start >= bytes.len() {
            warnings.push(format!(
                "entry[{position}] {} offset {offset:#x} is past the end",
                record.name
            ));
            entries.push(record);
            continue;
        }

        let end = offsets
            .iter()
            .copied()
            .filter(|value| *value > offset)
            .min()
            .map(|value| (value as usize).min(bytes.len()))
            .unwrap_or(bytes.len());
        let end = end.max(start);

        match walk(&bytes[start..end]) {
            Ok(commands) => {
                let mut at = offset;
                for command in &commands {
                    let opcode = *command.opcode();
                    let body = command.body();
                    record.commands += 1;
                    stats.commands += 1;

                    if !TEXT_OPCODES.contains(&opcode) {
                        at += 0x10 + *command.body_size();
                        continue;
                    }

                    let operands = operands(&body);
                    let text_id = operands.first().and_then(number).unwrap_or(NO_VALUE);
                    let speaker = operands.get(1).and_then(text);
                    let voice = operands.get(2).and_then(number).filter(|v| *v != NO_VALUE);
                    let resolved = pool.and_then(|pool| pool.get(text_id)).map(str::to_string);

                    stats.text_commands += 1;
                    if text_id == NO_VALUE {
                        stats.missing_text += 1;
                    } else {
                        ids.insert(text_id);
                        if resolved.is_none() {
                            stats.missing_text += 1;
                        }
                    }

                    record.lines.push(Line {
                        at,
                        opcode,
                        text_id,
                        speaker,
                        voice,
                        text: resolved,
                    });
                    at += 0x10 + *command.body_size();
                }
            }
            Err(error) => warnings.push(format!(
                "entry[{position}] {} at {offset:#x}: {error}",
                record.name
            )),
        }

        entries.push(record);
    }

    stats.unique_ids = ids.len() as u32;

    Ok(Dialogue {
        entries,
        stats,
        pool_path: pool.map(|pool| pool.path().to_path_buf()),
        pool_count: pool.map(|pool| pool.len()).unwrap_or(0),
        warnings,
    })
}

fn walk(stream: &[u8]) -> Result<Vec<kaitai::OptRc<schema::gen::rsc_command::RscCommand_Command>>> {
    let io = BytesReader::from(stream);
    let root = <RscCommand as KStruct>::read_into::<_, RscCommand>(&io, None, None)?;
    let commands = root.commands().to_vec();
    Ok(commands)
}

fn field_name(raw: &[u8]) -> String {
    let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

#[derive(Debug, Clone, PartialEq)]
enum Operand {
    Number(u32),
    Text(String),
}

fn number(operand: &Operand) -> Option<u32> {
    match operand {
        Operand::Number(value) => Some(*value),
        Operand::Text(_) => None,
    }
}

fn text(operand: &Operand) -> Option<String> {
    match operand {
        Operand::Number(_) => None,
        Operand::Text(value) => Some(value.clone()),
    }
}

fn operands(body: &[u8]) -> Vec<Operand> {
    const TAG_NUMBER: u32 = 3;
    const TAG_TEXT: u32 = 1;

    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 4 <= body.len() {
        let tag = u32::from_le_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]);
        pos += 4;
        match tag {
            TAG_NUMBER => {
                if pos + 4 > body.len() {
                    break;
                }
                let value =
                    u32::from_le_bytes([body[pos], body[pos + 1], body[pos + 2], body[pos + 3]]);
                out.push(Operand::Number(value));
                pos += 4;
            }
            TAG_TEXT => {
                let Some(length) = body[pos..].iter().position(|byte| *byte == 0) else {
                    break;
                };
                let raw = &body[pos..pos + length];
                out.push(Operand::Text(String::from_utf8_lossy(raw).into_owned()));
                pos = (pos + length + 1).div_ceil(4) * 4;
            }
            _ => break,
        }
    }
    out
}
