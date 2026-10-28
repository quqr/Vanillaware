//! ```text
//! +0x00  "@UTF"
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;
use super::text;

pub const MAGIC: &[u8] = b"@UTF";
const HEADER: usize = 0x20;
const MAX_TABLES: usize = 512;
const MAX_COLS_SHOWN: usize = 96;
const MAX_ROWS_SHOWN: usize = 64;

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub kind: u8,
    pub row_storage: bool,
    pub default_at: Option<usize>,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub at: usize,
    pub rows: usize,
    pub row_width: usize,
    pub row_count: usize,
    pub string_pool: usize,
    pub data_pool: usize,
    pub utf8: bool,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn rows_at(&self) -> usize {
        self.at + self.rows
    }

    pub fn computed_width(&self) -> usize {
        self.columns
            .iter()
            .filter(|col| col.row_storage)
            .map(|col| col.size)
            .sum()
    }
}

pub fn cell_size(kind: u8) -> usize {
    match kind {
        0 | 1 => 1,
        2 | 3 => 2,
        4 | 5 | 8 | 10 => 4,
        6 | 7 | 9 | 11 => 8,
        12 => 16,
        _ => 4,
    }
}

pub fn type_name(kind: u8) -> &'static str {
    match kind {
        0 => "u8",
        1 => "i8",
        2 => "u16",
        3 => "i16",
        4 => "u32",
        5 => "i32",
        6 => "u64",
        7 => "i64",
        8 => "f32",
        9 => "f64",
        10 => "string",
        11 => "raw",
        12 => "guid",
        _ => "?",
    }
}

pub fn parse_table(bytes: &[u8], at: usize) -> Option<Table> {
    if bytes.get(at..at + 4)? != MAGIC {
        return None;
    }
    let rows = rdr::u16_be_at(bytes, at + 0x0a)? as usize + 8;
    let string_pool = rdr::u32_be_at(bytes, at + 0x0c)? as usize + 8;
    let data_pool = rdr::u32_be_at(bytes, at + 0x10)? as usize + 8;
    let utf8 = rdr::u8_at(bytes, at + 0x09)? != 0;
    let column_count = rdr::u16_be_at(bytes, at + 0x18)? as usize;
    let row_width = rdr::u16_be_at(bytes, at + 0x1a)? as usize;
    let row_count = rdr::u32_be_at(bytes, at + 0x1c)? as usize;

    if column_count == 0 || column_count > 4096 || rows < HEADER {
        return None;
    }
    if at + rows + row_width.saturating_mul(row_count) > bytes.len() {
        return None;
    }
    if at + string_pool > bytes.len() {
        return None;
    }

    let mut columns = Vec::with_capacity(column_count);
    let mut cursor = at + HEADER;
    for _ in 0..column_count {
        let flag = rdr::u8_at(bytes, cursor)?;
        cursor += 1;
        let kind = flag & 0x0f;
        let mut name = String::new();
        if flag & 0x10 != 0 {
            let offset = rdr::u32_be_at(bytes, cursor)? as usize;
            cursor += 4;
            name = read_pool_string(bytes, at + string_pool, offset, utf8);
        }
        let mut default_at = None;
        if flag & 0x20 != 0 {
            default_at = Some(cursor);
            cursor += cell_size(kind);
        }
        let row_storage = flag & 0x40 != 0;
        columns.push(Column {
            name,
            kind,
            row_storage,
            default_at,
            size: if row_storage { cell_size(kind) } else { 0 },
        });
    }

    Some(Table {
        at,
        rows,
        row_width,
        row_count,
        string_pool: at + string_pool,
        data_pool: at + data_pool,
        utf8,
        columns,
    })
}

pub fn scan_tables(bytes: &[u8]) -> Vec<Table> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(found) = find(bytes, MAGIC, cursor) {
        if let Some(table) = parse_table(bytes, found) {
            out.push(table);
        }
        cursor = found + 1;
        if out.len() >= MAX_TABLES {
            break;
        }
    }
    out
}

fn find(bytes: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from + needle.len() > bytes.len() {
        return None;
    }
    bytes[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|index| from + index)
}

fn read_pool_string(bytes: &[u8], pool: usize, offset: usize, utf8: bool) -> String {
    let Some(start) = pool.checked_add(offset) else {
        return String::new();
    };
    if start >= bytes.len() {
        return String::new();
    }
    let rest = &bytes[start..];
    let end = rest
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(rest.len());
    decode(&rest[..end], utf8)
}

fn decode(raw: &[u8], utf8: bool) -> String {
    if utf8 {
        return text::utf8_or_sjis(raw);
    }
    text::sjis_slot(raw)
}

pub fn cell_text(bytes: &[u8], table: &Table, at: usize, column: &Column) -> String {
    let Some(slice) = bytes.get(at..at + column.size) else {
        return "—".to_string();
    };
    match column.kind {
        0 => slice[0].to_string(),
        1 => (slice[0] as i8).to_string(),
        2 => rdr::u16_be_at(slice, 0).unwrap_or(0).to_string(),
        3 => (rdr::u16_be_at(slice, 0).unwrap_or(0) as i16).to_string(),
        4 => rdr::u32_be_at(slice, 0).unwrap_or(0).to_string(),
        5 => (rdr::u32_be_at(slice, 0).unwrap_or(0) as i32).to_string(),
        6 => {
            let hi = rdr::u32_be_at(slice, 0).unwrap_or(0) as u64;
            let lo = rdr::u32_be_at(slice, 4).unwrap_or(0) as u64;
            ((hi << 32) | lo).to_string()
        }
        7 => {
            let hi = rdr::u32_be_at(slice, 0).unwrap_or(0) as u64;
            let lo = rdr::u32_be_at(slice, 4).unwrap_or(0) as u64;
            (((hi << 32) | lo) as i64).to_string()
        }
        8 => format!("{}", rdr::f32_be_at(slice, 0).unwrap_or(0.0)),
        9 => {
            let hi = rdr::u32_be_at(slice, 0).unwrap_or(0) as u64;
            let lo = rdr::u32_be_at(slice, 4).unwrap_or(0) as u64;
            format!("{}", f64::from_bits((hi << 32) | lo))
        }
        10 => {
            let offset = rdr::u32_be_at(slice, 0).unwrap_or(0) as usize;
            read_pool_string(bytes, table.string_pool, offset, table.utf8)
        }
        11 => {
            let size = rdr::u32_be_at(slice, 0).unwrap_or(0);
            let offset = rdr::u32_be_at(slice, 4).unwrap_or(0);
            let absolute = table.data_pool.saturating_add(offset as usize);
            let head = bytes
                .get(absolute..(absolute + 4).min(bytes.len()))
                .map(rdr::hex)
                .unwrap_or_default();
            format!("len={size} @{offset:#x} ({head})")
        }
        12 => rdr::hex(slice),
        _ => rdr::hex(slice),
    }
}

pub fn append(builder: &mut Builder, root: u32, bytes: &[u8]) -> usize {
    let tables = scan_tables(bytes);
    builder.field(root, "tables", tables.len().to_string());

    if tables.is_empty() {
        builder.warn("no parsable @UTF table found");
        builder.mark(root, DocMark::Error);
        return 0;
    }

    let first = &tables[0];
    let declared = rdr::u32_be_at(bytes, first.at + 4).unwrap_or(0) as usize;
    builder.check(
        "utf.first_table",
        "the first table sits at the start of the file",
        first.at == 0,
        Some(format!("@{:#x}", first.at)),
    );
    builder.check(
        "utf.table_size",
        "table length + 0x08 stays inside the file",
        declared + 8 <= bytes.len(),
        Some(format!("{declared:#x} + 8 ≤ {:#x}", bytes.len())),
    );

    let group = builder.array(root, Some("tables"), format!("Tables ({})", tables.len()));
    let mut width_ok = 0usize;
    for (index, table) in tables.iter().enumerate() {
        let key = format!("{index}");
        let node = builder.group(
            group,
            Some(&key),
            format!("Table[{index}] @0x{:x}", table.at),
        );
        builder.field(node, "offset", format!("0x{:x}", table.at));
        builder.field(node, "columns", table.columns.len().to_string());
        builder.field(node, "rows", table.row_count.to_string());
        builder.field(node, "rowWidth", table.row_width.to_string());

        let computed = table.computed_width();
        if computed == table.row_width {
            width_ok += 1;
        } else {
            builder.mark(node, DocMark::Warn).warn(format!(
                "table[{index}] row width {computed} != declared {}",
                table.row_width
            ));
        }

        let columns = builder.array(
            node,
            Some("columns"),
            format!("Columns ({})", table.columns.len()),
        );
        for (column_index, column) in table.columns.iter().take(MAX_COLS_SHOWN).enumerate() {
            let label = if column.name.is_empty() {
                format!("[{column_index}]")
            } else {
                column.name.clone()
            };
            let key = format!("c{column_index}");
            let value = format!(
                "{} · {} B{}",
                type_name(column.kind),
                column.size,
                if column.default_at.is_some() {
                    " · default value"
                } else {
                    ""
                }
            );
            builder.field(columns, &key, format!("{label}: {value}"));
        }

        let rows = builder.array(node, Some("rows"), format!("Rows ({})", table.row_count));
        let shown = table.row_count.min(MAX_ROWS_SHOWN);
        for row_index in 0..shown {
            let at = table.rows_at() + row_index * table.row_width;
            let row = builder.group(rows, Some(&row_index.to_string()), format!("[{row_index}]"));
            let mut cursor = at;
            for (column_index, column) in table.columns.iter().enumerate() {
                if !column.row_storage {
                    continue;
                }
                let key = format!("c{column_index}");
                let value = cell_text(bytes, table, cursor, column);
                if column.name.is_empty() {
                    builder.field(row, &key, value);
                } else {
                    builder.field(row, &column.name.clone(), value);
                }
                cursor += column.size;
            }
        }
        if table.row_count > shown {
            builder.field(node, "rowsShown", shown.to_string());
            builder.truncate();
        }
    }

    builder.check(
        "utf.row_width",
        "every table row width equals the sum of its column widths",
        width_ok == tables.len(),
        Some(format!("{width_ok}/{} tables", tables.len())),
    );

    tables.len()
}

pub fn build(bytes: &[u8], title: &str) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root(title);
    builder.field(root, "size", bytes.len().to_string());
    append(&mut builder, root, bytes);
    builder.finish()
}
