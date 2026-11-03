use super::arena::{Builder, DocArena, DocMark};
pub use crate::field::record_size;

use crate::field;

const MAX_RECORDS_SHOWN: usize = 64;
const MAX_GRIDS_SHOWN: usize = 64;

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("UFB");
    builder.field(root, "size", bytes.len().to_string());

    let Some(layout) = field::parse(bytes) else {
        builder.warn("file is shorter than 0x88 bytes");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    };
    let counts = layout.counts;
    let derived = layout.derived;

    let header = builder.group(root, Some("header"), "Header");
    builder.field(header, "records", counts.records.to_string());
    builder.field(header, "groupBlocks", counts.group_blocks.to_string());
    builder.field(header, "objects", counts.objects.to_string());
    builder.field(header, "modelObjects", counts.model_objects.to_string());
    builder.field(header, "panelObjects", counts.panel_objects.to_string());
    builder.field(header, "masks", counts.masks.to_string());
    builder.field(header, "fRecords", counts.f_records.to_string());
    builder.field(header, "words", counts.words.to_string());
    builder.field(header, "pairs", counts.pairs.to_string());
    builder.field(header, "samples", counts.samples.to_string());
    builder.field(header, "gNames", counts.groups.to_string());
    builder.field(header, "names", counts.names.to_string());

    let summary = builder.group(root, Some("summary"), "Summary");
    builder.field(summary, "layers", derived.layers.to_string());
    builder.field(summary, "panelLayers", derived.panel_layers.to_string());
    builder.field(summary, "modelLayers", derived.model_layers.to_string());
    builder.field(summary, "extBlocks", derived.ext_blocks.to_string());
    builder.field(summary, "groupEntries", derived.group_entries.to_string());
    builder.field(
        summary,
        "tileGrid",
        format!(
            "{} x {} = {}",
            field::COLUMNS,
            field::ROWS,
            field::COLUMNS as u64 * field::ROWS as u64
        ),
    );
    builder.field(
        summary,
        "worldGrid",
        format!(
            "{} x {} cells of {} world units",
            field::GRID_COLUMNS,
            field::GRID_ROWS,
            field::MASK_CELL
        ),
    );
    builder.field(
        summary,
        "worldOrigin",
        format!("x {}, z {}", field::WORLD_X0, field::WORLD_Z0),
    );
    builder.field(
        summary,
        "maskOrder",
        format!(
            "paint 0x6641 cells in ascending +0x34; {:#06x} masks go first and end up fully covered, which reproduces Field.ucb",
            field::MASK_UNDERLAY
        ),
    );
    builder.field(
        summary,
        "layout",
        format!(
            "records -> 0x{:x} · masks -> 0x{:x} · F -> 0x{:x} · strings -> 0x{:x}",
            layout
                .records
                .last()
                .map_or(0, |record| record.at + record.size),
            layout.masks.last().map_or(0, |mask| mask.at),
            layout.f_records.last().map_or(0, |record| record.at),
            layout.end
        ),
    );

    for check in field::checks(&layout, bytes.len()) {
        builder.check(check.id, check.what, check.ok, Some(check.detail));
    }

    let list = builder.array(
        root,
        Some("records"),
        format!("Records ({})", counts.records),
    );
    for record in layout.records.iter().take(MAX_RECORDS_SHOWN) {
        let node = builder.group(
            list,
            Some(&record.index.to_string()),
            format!("[{}] @0x{:x}", record.index, record.at),
        );
        builder.field(node, "offset", format!("0x{:x}", record.at));
        builder.field(node, "size", format!("0x{:x}", record.size));
        builder.field(node, "tile", format!("{}, {}", record.column, record.row));
        builder.field(node, "layers", record.layers.len().to_string());
        builder.field(node, "extBlocks", record.ext_count.to_string());
        builder.field(node, "groups", record.groups.len().to_string());
        builder.field(
            node,
            "bounds",
            format!(
                "({:.1}, {:.1}, {:.1})..({:.1}, {:.1}, {:.1})",
                record.min[0],
                record.min[1],
                record.min[2],
                record.max[0],
                record.max[1],
                record.max[2]
            ),
        );
        builder.bytes(
            node,
            Some("bytes"),
            "record",
            record.at as u64,
            record.size as u64,
        );
    }
    if counts.records as usize > MAX_RECORDS_SHOWN {
        builder.field(root, "recordsShown", MAX_RECORDS_SHOWN.to_string());
        builder.truncate();
    }

    let list = builder.array(root, Some("masks"), format!("Masks ({})", counts.masks));
    for (index, mask) in layout.masks.iter().take(MAX_GRIDS_SHOWN).enumerate() {
        let node = builder.group(
            list,
            Some(&index.to_string()),
            format!(
                "[{index}] {}x{}",
                mask.rect[2] - mask.rect[0] + 1,
                mask.rect[3] - mask.rect[1] + 1
            ),
        );
        builder.field(node, "offset", format!("0x{:x}", mask.at));
        builder.field(
            node,
            "world",
            format!(
                "({:.0}, {:.0})..({:.0}, {:.0})",
                mask.world[0], mask.world[2], mask.world[3], mask.world[5]
            ),
        );
        builder.field(
            node,
            "cells",
            format!(
                "({}, {})..({}, {})",
                mask.rect[0], mask.rect[1], mask.rect[2], mask.rect[3]
            ),
        );
        builder.field(node, "marked", mask.marked.to_string());
        builder.field(
            node,
            "group",
            if mask.group < 0 {
                "-".to_string()
            } else {
                mask.group.to_string()
            },
        );
        builder.bytes(
            node,
            Some("bytes"),
            "mask",
            mask.at as u64,
            (field::MASK_HEAD + mask.cells as usize * 4) as u64,
        );
    }
    if counts.masks as usize > MAX_GRIDS_SHOWN {
        builder.field(root, "masksShown", MAX_GRIDS_SHOWN.to_string());
        builder.truncate();
    }

    let list = builder.array(
        root,
        Some("fRecords"),
        format!("F records ({})", counts.f_records),
    );
    for (index, record) in layout.f_records.iter().enumerate() {
        let node = builder.group(list, Some(&index.to_string()), format!("[{index}]"));
        builder.field(node, "offset", format!("0x{:x}", record.at));
        builder.field(
            node,
            "rect",
            format!(
                "x {:.0}..{:.0}, z {:.0}..{:.0}",
                record.rect[0], record.rect[2], record.rect[1], record.rect[3]
            ),
        );
        builder.field(
            node,
            "tags",
            record
                .tags
                .iter()
                .map(|tag| tag.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    let list = builder.array(
        root,
        Some("pairs"),
        format!("Resource pairs ({})", counts.pairs),
    );
    for (index, pair) in layout.pairs.iter().enumerate() {
        let node = builder.group(list, Some(&index.to_string()), pair.model.clone());
        builder.field(node, "model", pair.model.clone());
        builder.field(node, "texture", pair.texture.clone());
    }

    let list = builder.array(
        root,
        Some("samples"),
        format!("Sample names ({})", layout.samples.len()),
    );
    for (index, name) in layout.samples.iter().enumerate() {
        builder.text(list, &index.to_string(), name.clone(), name.clone(), None);
    }
    let list = builder.array(
        root,
        Some("gNames"),
        format!("Group names ({})", layout.group_names.len()),
    );
    for (index, name) in layout.group_names.iter().enumerate() {
        builder.text(list, &index.to_string(), name.clone(), name.clone(), None);
    }
    let list = builder.array(
        root,
        Some("names"),
        format!("Names ({})", layout.names.len()),
    );
    for (index, name) in layout.names.iter().enumerate() {
        builder.text(list, &index.to_string(), name.clone(), name.clone(), None);
    }

    builder.finish()
}
