use crate::table::Table;
use crate::{CpkEntry, Error, Result};

pub struct Locate {
    pub toc_offset: u64,
    pub content_offset: u64,
    pub content_base_is_toc: bool,
}

pub fn locate(header: &Table) -> Result<Locate> {
    let toc_offset = value(header, "TocOffset")?;
    let declared_content = value(header, "ContentOffset")?;

    let content_base_is_toc = toc_offset < declared_content;
    let content_offset = if content_base_is_toc {
        toc_offset
    } else {
        declared_content
    };

    Ok(Locate {
        toc_offset,
        content_offset,
        content_base_is_toc,
    })
}

pub fn entries(toc: &Table, content_offset: u64) -> Result<Vec<CpkEntry>> {
    let name = toc
        .column("FileName")
        .ok_or(Error::MissingColumn("FileName"))?;
    let size = toc
        .column("FileSize")
        .ok_or(Error::MissingColumn("FileSize"))?;
    let extract = toc
        .column("ExtractSize")
        .ok_or(Error::MissingColumn("ExtractSize"))?;
    let offset = toc
        .column("FileOffset")
        .ok_or(Error::MissingColumn("FileOffset"))?;
    let dir = toc.column("DirName");
    let user = toc.column("UserString");

    let mut entries = Vec::with_capacity(toc.rows());
    for row in 0..toc.rows() {
        let Some(file_name) = toc.string(name, row) else {
            continue;
        };
        if file_name.is_empty() {
            continue;
        }

        entries.push(CpkEntry {
            dir: dir
                .and_then(|column| toc.string(column, row))
                .map(|text| normalize(&text))
                .unwrap_or_default(),
            name: file_name,
            size: toc.uint(size, row).unwrap_or(0),
            extract_size: toc.uint(extract, row).unwrap_or(0),
            offset: toc
                .uint(offset, row)
                .unwrap_or(0)
                .saturating_add(content_offset),
            user_string: user
                .and_then(|column| toc.string(column, row))
                .filter(|text| !text.is_empty()),
        });
    }

    Ok(entries)
}

pub fn value(table: &Table, name: &'static str) -> Result<u64> {
    let column = table.column(name).ok_or(Error::MissingColumn(name))?;
    table.uint(column, 0).ok_or(Error::MissingColumn(name))
}

pub fn normalize(text: &str) -> String {
    text.replace('\\', "/").trim_matches('/').trim().to_string()
}
