use std::ops::Range;

use encoding_rs::{Encoding, SHIFT_JIS, UTF_8};

use crate::{Error, Result};

pub const MAGIC: &[u8; 4] = b"@UTF";

const HEADER_LEN: usize = 0x20;
const BASE: usize = 0x08;

const FLAG_NAME: u8 = 0x10;
const FLAG_DEFAULT: u8 = 0x20;
const FLAG_ROW: u8 = 0x40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnKind {
    Byte,
    SByte,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    F64,
    Str,
    Raw,
    Guid,
}

impl ColumnKind {
    fn from_bits(bits: u8) -> Result<Self> {
        Ok(match bits {
            0 => ColumnKind::Byte,
            1 => ColumnKind::SByte,
            2 => ColumnKind::U16,
            3 => ColumnKind::I16,
            4 => ColumnKind::U32,
            5 => ColumnKind::I32,
            6 => ColumnKind::U64,
            7 => ColumnKind::I64,
            8 => ColumnKind::F32,
            9 => ColumnKind::F64,
            10 => ColumnKind::Str,
            11 => ColumnKind::Raw,
            12 => ColumnKind::Guid,
            _ => {
                return Err(Error::BadTable {
                    reason: "unknown column type",
                })
            }
        })
    }

    pub fn width(self) -> usize {
        match self {
            ColumnKind::Byte | ColumnKind::SByte => 1,
            ColumnKind::U16 | ColumnKind::I16 => 2,
            ColumnKind::U32 | ColumnKind::I32 | ColumnKind::F32 | ColumnKind::Str => 4,
            ColumnKind::U64 | ColumnKind::I64 | ColumnKind::F64 | ColumnKind::Raw => 8,
            ColumnKind::Guid => 16,
        }
    }
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub kind: ColumnKind,
    pub in_row: bool,
    pub offset: usize,
    width: usize,
    default: Option<Range<usize>>,
}

pub struct Table {
    bytes: Vec<u8>,
    encoding: &'static Encoding,
    rows_at: usize,
    strings_at: usize,
    row_width: usize,
    row_count: usize,
    in_row_width: usize,
    columns: Vec<Column>,
}

impl Table {
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::BadTable {
                reason: "table header is truncated",
            });
        }
        if &bytes[..MAGIC.len()] != MAGIC {
            return Err(Error::BadTable {
                reason: "magic is not @UTF",
            });
        }

        let rows_at = crate::be_u16(&bytes, 0x0A).ok_or(Error::BadTable {
            reason: "row offset is missing",
        })? as usize
            + BASE;
        let strings_at = crate::be_i32(&bytes, 0x0C).ok_or(Error::BadTable {
            reason: "string pool offset is missing",
        })? as usize
            + BASE;
        let column_count = crate::be_u16(&bytes, 0x18).ok_or(Error::BadTable {
            reason: "column count is missing",
        })? as usize;
        let row_width = crate::be_u16(&bytes, 0x1A).ok_or(Error::BadTable {
            reason: "row width is missing",
        })? as usize;
        let row_count = crate::be_i32(&bytes, 0x1C).ok_or(Error::BadTable {
            reason: "row count is missing",
        })?;

        if row_count < 0 {
            return Err(Error::BadTable {
                reason: "row count is negative",
            });
        }
        if strings_at >= bytes.len() {
            return Err(Error::BadTable {
                reason: "string pool offset is outside the table",
            });
        }
        let encoding = if bytes[0x09] == 0 { SHIFT_JIS } else { UTF_8 };

        let mut columns = Vec::with_capacity(column_count);
        let mut cursor = HEADER_LEN;
        let mut offset = 0usize;

        for _ in 0..column_count {
            let flags = *bytes.get(cursor).ok_or(Error::BadTable {
                reason: "column descriptor runs past the table",
            })?;
            cursor += 1;

            let kind = ColumnKind::from_bits(flags & 0x0F)?;
            let width = kind.width();

            let name = if flags & FLAG_NAME != 0 {
                let at = crate::be_u32(&bytes, cursor).ok_or(Error::BadTable {
                    reason: "column name offset is truncated",
                })?;
                cursor += 4;
                read_pool_string(&bytes, strings_at, at as usize, encoding).ok_or(
                    Error::BadTable {
                        reason: "column name offset is outside the string pool",
                    },
                )?
            } else {
                String::new()
            };

            let default = if flags & FLAG_DEFAULT != 0 {
                let at = cursor;
                if at + width > bytes.len() {
                    return Err(Error::BadTable {
                        reason: "column default value is truncated",
                    });
                }
                cursor += width;
                Some(at..at + width)
            } else {
                None
            };

            let in_row = flags & FLAG_ROW != 0;
            columns.push(Column {
                name,
                kind,
                in_row,
                offset,
                width,
                default,
            });
            if in_row {
                offset += width;
            }
        }

        if offset > row_width {
            return Err(Error::BadTable {
                reason: "row storage columns are wider than the declared row width",
            });
        }
        if row_count as usize * row_width + rows_at > bytes.len() {
            return Err(Error::BadTable {
                reason: "rows run past the end of the table",
            });
        }

        Ok(Self {
            bytes,
            encoding,
            rows_at,
            strings_at,
            row_width,
            row_count: row_count as usize,
            in_row_width: offset,
            columns,
        })
    }

    pub fn rows(&self) -> usize {
        self.row_count
    }

    pub fn row_width(&self) -> usize {
        self.row_width
    }

    pub fn in_row_width(&self) -> usize {
        self.in_row_width
    }

    pub fn padding(&self) -> usize {
        self.row_width.saturating_sub(self.in_row_width)
    }

    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|column| column.name == name)
    }

    pub fn column_names(&self) -> Vec<&str> {
        self.columns
            .iter()
            .map(|column| column.name.as_str())
            .collect()
    }

    pub fn string(&self, column: &Column, row: usize) -> Option<String> {
        if column.kind != ColumnKind::Str {
            return None;
        }
        let at = crate::be_u32(self.cell(column, row)?, 0)? as usize;
        read_pool_string(&self.bytes, self.strings_at, at, self.encoding)
    }

    pub fn uint(&self, column: &Column, row: usize) -> Option<u64> {
        let bytes = self.cell(column, row)?;
        Some(match column.kind {
            ColumnKind::Byte => bytes[0] as u64,
            ColumnKind::SByte => bytes[0] as i8 as i64 as u64,
            ColumnKind::U16 => u16::from_be_bytes([bytes[0], bytes[1]]) as u64,
            ColumnKind::I16 => i16::from_be_bytes([bytes[0], bytes[1]]) as i64 as u64,
            ColumnKind::U32 => crate::be_u32(bytes, 0)? as u64,
            ColumnKind::I32 => crate::be_i32(bytes, 0)? as i64 as u64,
            ColumnKind::U64 => crate::be_u64(bytes, 0)?,
            ColumnKind::I64 => crate::be_u64(bytes, 0)? as i64 as u64,
            _ => return None,
        })
    }

    pub fn cell(&self, column: &Column, row: usize) -> Option<&[u8]> {
        if row >= self.row_count {
            return None;
        }
        let range = if column.in_row {
            let at = self.rows_at + row * self.row_width + column.offset;
            at..at + column.width
        } else {
            column.default.clone()?
        };
        self.bytes.get(range)
    }
}

fn read_pool_string(
    bytes: &[u8],
    strings_at: usize,
    offset: usize,
    encoding: &'static Encoding,
) -> Option<String> {
    let start = strings_at.checked_add(offset)?;
    if start > bytes.len() {
        return None;
    }
    let rest = &bytes[start..];
    let end = rest.iter().position(|byte| *byte == 0)?;
    let (text, _, _) = encoding.decode(&rest[..end]);
    Some(text.into_owned())
}
