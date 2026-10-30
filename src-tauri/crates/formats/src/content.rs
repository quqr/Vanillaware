use std::collections::BTreeMap;

use super::document::rdr;

const MAX_BIT_COLUMNS: usize = 768;

pub struct Section {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total: u64,
}

impl Section {
    pub(crate) fn new(title: &str, columns: &[&str], rows: Vec<Vec<String>>, total: u64) -> Self {
        Self {
            title: title.to_string(),
            columns: columns.iter().map(|value| value.to_string()).collect(),
            rows,
            total,
        }
    }
}

fn fold_bits(values: &[u8], width: usize, max_columns: usize) -> (Vec<u8>, usize, usize) {
    let rows = values.len() / width.max(1);
    if rows == 0 {
        return (Vec::new(), width, 0);
    }
    let step = rows.div_ceil(max_columns).max(1);
    let columns = rows.div_ceil(step);
    let mut out = vec![0u8; columns * width];
    for row in 0..rows {
        let column = row / step;
        let source = row * width;
        for bit in 0..width {
            out[bit * columns + column] |= values[source + bit];
        }
    }
    (out, columns, width)
}

pub struct GrayMap {
    pub width: u32,
    pub height: u32,
    pub values: Vec<u8>,
    pub legend: String,
}

pub struct Series {
    pub label: String,
    pub points: Vec<f32>,
    pub note: Option<String>,
}

pub struct ColorMap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub legend: String,
}

pub struct Projection {
    pub title: String,
    pub note: Option<String>,
    pub sections: Vec<Section>,
    pub bitmap: Option<GrayMap>,
    pub color: Option<ColorMap>,
    pub series: Option<Series>,
    pub text: Option<Vec<String>>,
}

impl Projection {
    pub(crate) fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            note: None,
            sections: Vec::new(),
            bitmap: None,
            color: None,
            series: None,
            text: None,
        }
    }
}

pub fn project(bytes: &[u8], ext: &str) -> Option<Projection> {
    match ext {
        "drg" => Some(drg(bytes)),
        "vs2" => Some(vs2(bytes)),
        "nnd" => Some(nnd(bytes)),
        "ucb" => Some(ucb(bytes)),
        "scb" => Some(scb(bytes)),
        "inc" => Some(inc(bytes)),
        "mpd" => Some(mpd(bytes)),
        "ctb" => Some(ctb(bytes)),
        "fsb" => Some(fsb(bytes)),
        "acf" => Some(acf(bytes)),
        "ufb" => Some(ufb(bytes)),
        "oeb" => Some(oeb(bytes)),
        "orb" => Some(orb(bytes)),
        "bst" => Some(bst(bytes)),
        _ => None,
    }
}

pub(crate) fn hsv(hue: f32, saturation: f32, value: f32) -> [u8; 3] {
    let sector = (hue.rem_euclid(360.0)) / 60.0;
    let chroma = value * saturation;
    let second = chroma * (1.0 - ((sector % 2.0) - 1.0).abs());
    let (red, green, blue) = match sector as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let base = value - chroma;
    let scale = |channel: f32| (((channel + base) * 255.0).clamp(0.0, 255.0)) as u8;
    [scale(red), scale(green), scale(blue)]
}

pub(crate) fn area_hue(id: u32) -> f32 {
    (id as f32 * 137.508).rem_euclid(360.0)
}

fn put(rgba: &mut [u8], at: usize, color: [u8; 3]) {
    rgba[at] = color[0];
    rgba[at + 1] = color[1];
    rgba[at + 2] = color[2];
    rgba[at + 3] = 255;
}

fn push_bits(values: &mut Vec<u8>, word: u64, width: u32) {
    for bit in 0..width {
        let on = (word >> (width - 1 - bit)) & 1 == 1;
        values.push(if on { 255 } else { 0 });
    }
}

fn drg(bytes: &[u8]) -> Projection {
    let db_count = rdr::u32_at(bytes, 4).unwrap_or(0) as usize;
    let cmd_count = rdr::u32_at(bytes, 8).unwrap_or(0) as usize;
    let mut out = Projection::new("DbReg debug parameter registry");
    out.note = Some(format!("{db_count} databases / {cmd_count} commands"));

    let mut rows = Vec::new();
    for index in 0..db_count.min(200) {
        let at = 0x38 + index * 0x90;
        if at + 0x90 > bytes.len() {
            break;
        }
        rows.push(vec![
            index.to_string(),
            rdr::c_string(&bytes[at..at + 64]),
            rdr::c_string(&bytes[at + 64..at + 96]),
            rdr::utf8_string(&bytes[at + 96..at + 128]),
            format!(
                "{} {} {} {}",
                rdr::u32_at(bytes, at + 0x80).unwrap_or(0),
                rdr::u32_at(bytes, at + 0x84).unwrap_or(0),
                rdr::u32_at(bytes, at + 0x88).unwrap_or(0),
                rdr::u32_at(bytes, at + 0x8c).unwrap_or(0)
            ),
        ]);
    }
    if !rows.is_empty() {
        out.sections.push(Section::new(
            "Databases",
            &["#", "raw path", "name", "label", "u32 ×4"],
            rows,
            db_count as u64,
        ));
    }

    let cmd_at = rdr::u32_at(bytes, 0x18).unwrap_or(0) as usize;
    if cmd_count > 0 && cmd_at > 0 {
        let start = cmd_at + 0x10;
        let mut rows = Vec::new();
        for index in 0..cmd_count.min(2000) {
            let at = start + index * 0xD0;
            if at + 0x40 >= bytes.len() {
                break;
            }
            let label_end = (at + 0xD0).min(bytes.len());
            rows.push(vec![
                index.to_string(),
                rdr::c_string(&bytes[at..at + 64]),
                rdr::utf8_string(&bytes[(at + 64).min(label_end)..label_end]),
            ]);
        }
        out.sections.push(Section::new(
            "Commands",
            &["#", "command", "description"],
            rows,
            cmd_count as u64,
        ));
    } else if bytes.len() > 0x38 {
        let count = (bytes.len() - 0x38) / 4;
        let points: Vec<f32> = (0..count.min(4096))
            .filter_map(|index| rdr::f32_at(bytes, 0x38 + index * 4))
            .filter(|value| value.is_finite())
            .collect();
        out.note = Some(format!(
            "no database or command records; body is {count} f32 values"
        ));
        out.series = Some(Series {
            label: "f32 values".to_string(),
            points,
            note: Some("file order".to_string()),
        });
    }
    out
}

fn vs2(bytes: &[u8]) -> Projection {
    let count = rdr::u32_at(bytes, 0).unwrap_or(0) as usize;
    let mut out = Projection::new("VoiceSpeak voice table");
    let ids_at = 0x10;
    let mask_at = ids_at + count * 4;
    let expected = mask_at + count * 8;
    out.note = Some(format!(
        "{count} entries; 16 + {count}×4 + {count}×8 = {expected} (file {})",
        bytes.len()
    ));

    let mut rows = Vec::new();
    let mut values = Vec::with_capacity(count.min(20_000) * 64);
    for index in 0..count {
        let id = rdr::u32_at(bytes, ids_at + index * 4).unwrap_or(0);
        let mask = rdr::u64_at(bytes, mask_at + index * 8).unwrap_or(0);
        if values.len() < 20_000 * 64 {
            push_bits(&mut values, mask, 64);
        }
        if index < 200 {
            rows.push(vec![
                index.to_string(),
                id.to_string(),
                format!("{mask:016x}"),
                mask.count_ones().to_string(),
            ]);
        }
    }
    out.sections.push(Section::new(
        "Entries",
        &["#", "voice id", "flags", "bits set"],
        rows,
        count as u64,
    ));
    let (values, columns, bits) = fold_bits(&values, 64, MAX_BIT_COLUMNS);
    out.bitmap = Some(GrayMap {
        width: columns as u32,
        height: bits as u32,
        values,
        legend: format!(
            "x = {columns} segments over {count} entries, y = flag bit 63 (top) .. 0 (bottom); white means the bit was set in that segment"
        ),
    });
    out
}

fn nnd(bytes: &[u8]) -> Projection {
    let words = rdr::u32_at(bytes, 4).unwrap_or(0) as usize;
    let units = (words / 3).min(200_000);
    let mut out = Projection::new("NodeNetwork");
    out.note = Some(format!(
        "{units} triples; the third word expands to 32 bits, the second is a numeric series"
    ));

    let mut head = Vec::new();
    for index in 0..6 {
        head.push(vec![
            format!("+{:#04x}", 0x04 + index * 4),
            rdr::u32_at(bytes, 0x04 + index * 4)
                .unwrap_or(0)
                .to_string(),
        ]);
    }
    out.sections
        .push(Section::new("Header", &["offset", "value"], head, 6));

    let mut values = Vec::with_capacity(units * 32);
    let mut series = Vec::with_capacity(units);
    for index in 0..units {
        let at = 0x18 + index * 12;
        let third = rdr::u32_at(bytes, at + 8).unwrap_or(0) as u64;
        push_bits(&mut values, third, 32);
        series.push(rdr::f32_at(bytes, at + 4).unwrap_or(0.0));
    }
    let (values, columns, bits) = fold_bits(&values, 32, MAX_BIT_COLUMNS);
    out.bitmap = Some(GrayMap {
        width: columns as u32,
        height: bits as u32,
        values,
        legend: format!(
            "x = {columns} segments over {units} triples, y = third word bit 31 (top) .. 0 (bottom); white means the bit was set in that segment"
        ),
    });
    out.series = Some(Series {
        label: "second word".to_string(),
        points: series,
        note: Some("by group index; float in 0..128".to_string()),
    });

    let names = nnd_tail_names(bytes, 0x18 + words * 4);
    if !names.is_empty() {
        let rows: Vec<Vec<String>> = names
            .iter()
            .enumerate()
            .map(|(index, name)| vec![index.to_string(), name.clone()])
            .collect();
        let total = rows.len() as u64;
        out.sections.push(Section::new(
            "Trailing place names",
            &["#", "name"],
            rows,
            total,
        ));
    }
    out
}

fn nnd_tail_names(bytes: &[u8], from: usize) -> Vec<String> {
    if from >= bytes.len() {
        return Vec::new();
    }
    let tail = &bytes[from..];
    let limit = tail.len().min(8 * 1024) / 4 * 4;
    let mut best: Vec<String> = Vec::new();
    for start in (0..limit).step_by(4) {
        let mut names = Vec::new();
        let mut cursor = start;
        while cursor + 64 <= tail.len() {
            let slot = &tail[cursor..cursor + 64];
            let end = slot.iter().position(|value| *value == 0).unwrap_or(64);
            if end == 0 || slot[end..].iter().any(|value| *value != 0) {
                break;
            }
            match std::str::from_utf8(&slot[..end]) {
                Ok(text) if !text.is_empty() && !text.chars().any(|ch| ch.is_control()) => {
                    names.push(text.to_string());
                }
                _ => break,
            }
            cursor += 64;
        }
        if names.len() > best.len() {
            best = names;
        }
    }
    best.truncate(256);
    best
}

fn fsb(bytes: &[u8]) -> Projection {
    let mut out = Projection::new("FSB scene package");

    let mut blocks: Vec<(&[u8], usize, usize)> = Vec::new();
    let mut pos = 0usize;
    while pos + 8 <= bytes.len() {
        let magic = &bytes[pos..pos + 4];
        let size = rdr::u32_at(bytes, pos + 4).unwrap_or(0) as usize;
        if size < 8 || pos + size > bytes.len() {
            break;
        }
        blocks.push((magic, pos, size));
        pos += size;
    }
    out.note = Some(format!(
        "{} chunks, stopped at {pos} of {} bytes",
        blocks.len(),
        bytes.len()
    ));

    let mut table = Vec::new();
    let mut resources: Vec<String> = Vec::new();
    let mut catalog: Vec<Vec<String>> = Vec::new();
    let mut scene_note = String::new();

    for (magic, at, size) in &blocks {
        let body = at + 8;
        let end = at + size;
        let detail = match *magic {
            b"HEAD" => {
                let version = rdr::u32_at(bytes, body).unwrap_or(0);
                let scale = rdr::f32_at(bytes, body + 4).unwrap_or(0.0);
                format!("version {version:#04x} / constant {scale:.3}")
            }
            b"RESL" => {
                let count = rdr::u32_at(bytes, body).unwrap_or(0) as usize;
                let mut cursor = body + 4;
                for _ in 0..count.min(256) {
                    let Some(len) = rdr::u32_at(bytes, cursor) else {
                        break;
                    };
                    let len = len as usize;
                    if len == 0 || cursor + 4 + len > end {
                        break;
                    }
                    let raw = &bytes[cursor + 4..cursor + 4 + len];
                    let text = raw.split(|value| *value == 0).next().unwrap_or(&[]);
                    resources.push(String::from_utf8_lossy(text).to_string());
                    cursor += 4 + len;
                }
                format!("{count} resources")
            }
            b"CTLG" => {
                let count = rdr::u32_at(bytes, body).unwrap_or(0) as usize;
                for index in 0..count.min(512) {
                    let row = body + 4 + index * 16;
                    if row + 16 > end {
                        break;
                    }
                    catalog.push(vec![
                        index.to_string(),
                        rdr::u32_at(bytes, row).unwrap_or(0).to_string(),
                        rdr::u32_at(bytes, row + 4).unwrap_or(0).to_string(),
                        rdr::u32_at(bytes, row + 8).unwrap_or(0).to_string(),
                        rdr::u32_at(bytes, row + 12).unwrap_or(0).to_string(),
                    ]);
                }
                format!("{count} entries")
            }
            b"SCNL" => {
                let count = rdr::u32_at(bytes, body).unwrap_or(0);
                let stride = rdr::u32_at(bytes, body + 4).unwrap_or(0);
                scene_note = format!(
                    "{} segments × {stride} bytes; first segment {}",
                    count,
                    rdr::c_string(&bytes[(body + 8).min(end)..(body + 72).min(end)])
                );
                scene_note.clone()
            }
            _ => format!("{} B", size - 8),
        };
        table.push(vec![
            rdr::ascii(magic),
            format!("{at:#x}"),
            size.to_string(),
            detail,
        ]);
    }

    out.sections.push(Section::new(
        "Chunks",
        &["chunk", "offset", "size", "summary"],
        table,
        blocks.len() as u64,
    ));

    if !resources.is_empty() {
        let rows: Vec<Vec<String>> = resources
            .iter()
            .enumerate()
            .map(|(index, name)| vec![index.to_string(), name.clone()])
            .collect();
        let total = rows.len() as u64;
        out.sections.push(Section::new(
            "External resources",
            &["#", "file name"],
            rows,
            total,
        ));
    }

    if !catalog.is_empty() {
        let total = catalog.len() as u64;
        out.sections.push(Section::new(
            "Directories",
            &["#", "field 1", "field 2", "field 3", "index"],
            catalog,
            total,
        ));
    }

    if !scene_note.is_empty() {
        out.sections.push(Section::new(
            "Scene",
            &["item", "value"],
            vec![vec!["note".to_string(), scene_note]],
            1,
        ));
    }
    out
}

fn oeb(bytes: &[u8]) -> Projection {
    use super::document::oeb;
    let slots = oeb::slots(bytes);
    let resources = oeb::resources(bytes);
    let mut out = Projection::new("OEB character placement");
    out.note = Some(format!(
        "{} placements / {} external resources (.mbs and .ftx pair up)",
        slots.len(),
        resources.len()
    ));

    if !slots.is_empty() {
        let rows: Vec<Vec<String>> = slots
            .iter()
            .map(|slot| {
                vec![
                    slot.caption.clone(),
                    slot.label.clone(),
                    format!("{:.1}", slot.position[0]),
                    format!("{:.1}", slot.position[1]),
                    format!("{:.1}", slot.position[2]),
                    format!("{:.2}", slot.scale[0]),
                ]
            })
            .collect();
        let total = rows.len() as u64;
        out.sections.push(Section::new(
            "Placements",
            &["name", "character", "X", "Y", "Z", "scale"],
            rows,
            total,
        ));

        let min_x = slots.iter().map(|s| s.position[0]).fold(f32::MAX, f32::min);
        let max_x = slots.iter().map(|s| s.position[0]).fold(f32::MIN, f32::max);
        let min_y = slots.iter().map(|s| s.position[1]).fold(f32::MAX, f32::min);
        let max_y = slots.iter().map(|s| s.position[1]).fold(f32::MIN, f32::max);
        let size = 256usize;
        let mut values = vec![0u8; size * size];
        for slot in &slots {
            let x = scale(slot.position[0], min_x, max_x, size);
            let y = scale(slot.position[1], min_y, max_y, size);
            for (dx, dy) in [(0i32, 0i32), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                let px = (x as i32 + dx).clamp(0, size as i32 - 1) as usize;
                let py = (y as i32 + dy).clamp(0, size as i32 - 1) as usize;
                values[py * size + px] = 255;
            }
        }
        out.bitmap = Some(GrayMap {
            width: size as u32,
            height: size as u32,
            values,
            legend: format!(
                "top view: x = X ({min_x:.0}..{max_x:.0}), y = Y ({min_y:.0}..{max_y:.0}); white = one character"
            ),
        });
    }

    if !resources.is_empty() {
        let rows: Vec<Vec<String>> = resources
            .iter()
            .enumerate()
            .map(|(index, name)| vec![index.to_string(), name.clone()])
            .collect();
        let total = rows.len() as u64;
        out.sections
            .push(Section::new("Resources", &["#", "path"], rows, total));
    }
    out
}

fn scale(value: f32, low: f32, high: f32, size: usize) -> usize {
    if !value.is_finite() || high <= low {
        return size / 2;
    }
    let ratio = ((value - low) / (high - low)).clamp(0.0, 1.0);
    ((1.0 - ratio) * (size - 1) as f32) as usize
}

fn orb(bytes: &[u8]) -> Projection {
    use super::document::orb;
    let cameras = orb::cameras(bytes);
    let resources = orb::resources(bytes);
    let mut out = Projection::new("ORB round cameras");
    out.note = Some(format!(
        "{} cameras / {} resource paths",
        cameras.len(),
        resources.len()
    ));

    if !cameras.is_empty() {
        let rows: Vec<Vec<String>> = cameras
            .iter()
            .enumerate()
            .map(|(index, camera)| {
                vec![
                    index.to_string(),
                    camera.stage.clone(),
                    camera.label.clone(),
                    format!("{:.1}", camera.position[0]),
                    format!("{:.1}", camera.position[1]),
                    format!("{:.1}", camera.position[2]),
                ]
            })
            .collect();
        let total = rows.len() as u64;
        out.sections.push(Section::new(
            "Cameras",
            &["#", "stage", "index", "X", "Y", "Z"],
            rows,
            total,
        ));
    }

    if !resources.is_empty() {
        let rows: Vec<Vec<String>> = resources
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let short = path
                    .rsplit_once("Data\\")
                    .map(|(_, rest)| rest.to_string())
                    .unwrap_or_else(|| path.clone());
                vec![index.to_string(), short, path.clone()]
            })
            .collect();
        let total = rows.len() as u64;
        out.sections.push(Section::new(
            "Resources",
            &["#", "relative path", "raw path"],
            rows,
            total,
        ));
    }
    out
}

fn bst(bytes: &[u8]) -> Projection {
    use super::document::bst;
    let stage = bst::parse(bytes);
    let mut out = Projection::new("BST battle stage");

    out.note = Some(format!(
        "{} records / {} collision plates / {} terrain blocks",
        stage.records.len(),
        stage.plates.len(),
        stage.quads.len()
    ));

    let head = vec![
        vec![
            "magic".to_string(),
            rdr::ascii(&bytes[..4.min(bytes.len())]),
        ],
        vec![
            "stage counts".to_string(),
            rdr::u32_at(bytes, 0x10).unwrap_or(0).to_string(),
        ],
        vec!["Resources".to_string(), stage.names.join(" / ")],
        vec!["records".to_string(), stage.records.len().to_string()],
        vec![
            "closure".to_string(),
            if stage.unknown_tag.is_none() && stage.end == bytes.len() {
                format!("value stream reached EOF ({:#x})", stage.end)
            } else {
                format!("stopped at {:#x} / {:#x}", stage.end, bytes.len())
            },
        ],
    ];
    out.sections
        .push(Section::new("Header", &["item", "value"], head, 5));

    if !stage.plates.is_empty() {
        let rows: Vec<Vec<String>> = stage
            .plates
            .iter()
            .map(|plate| {
                vec![
                    plate.id.to_string(),
                    format!("{:.1}", plate.position[0]),
                    format!("{:.1}", plate.position[2]),
                    plate.corners.len().to_string(),
                    plate
                        .corners
                        .iter()
                        .map(|value| value.to_string())
                        .collect::<Vec<_>>()
                        .join(" "),
                ]
            })
            .collect();
        let total = rows.len() as u64;
        out.sections.push(Section::new(
            "Collision plates",
            &["id", "X", "Z", "corner count", "corner indices"],
            rows,
            total,
        ));
    }

    if !stage.quads.is_empty() {
        let rows: Vec<Vec<String>> = stage
            .quads
            .iter()
            .map(|quad| {
                vec![
                    quad.name.clone(),
                    format!("({:.0}, {:.0})", quad.corners[0][0], quad.corners[0][2]),
                    format!("({:.0}, {:.0})", quad.corners[1][0], quad.corners[1][2]),
                    format!("({:.0}, {:.0})", quad.corners[2][0], quad.corners[2][2]),
                    format!("({:.0}, {:.0})", quad.corners[3][0], quad.corners[3][2]),
                ]
            })
            .collect();
        let total = rows.len() as u64;
        out.sections.push(Section::new(
            "Terrain blocks",
            &["name", "corner 0", "corner 1", "corner 2", "corner 3"],
            rows,
            total,
        ));
    }

    let mut xs: Vec<f32> = Vec::new();
    let mut zs: Vec<f32> = Vec::new();
    for quad in &stage.quads {
        for corner in quad.corners {
            xs.push(corner[0]);
            zs.push(corner[2]);
        }
    }
    for plate in &stage.plates {
        xs.push(plate.position[0]);
        zs.push(plate.position[2]);
    }
    let finite: Vec<(f32, f32)> = xs
        .iter()
        .copied()
        .zip(zs.iter().copied())
        .filter(|(x, z)| x.is_finite() && z.is_finite())
        .collect();
    if finite.len() >= 4 {
        let min_x = finite.iter().map(|p| p.0).fold(f32::MAX, f32::min);
        let max_x = finite.iter().map(|p| p.0).fold(f32::MIN, f32::max);
        let min_z = finite.iter().map(|p| p.1).fold(f32::MAX, f32::min);
        let max_z = finite.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        let size = 448usize;
        let span_x = (max_x - min_x).max(1.0);
        let span_z = (max_z - min_z).max(1.0);
        let project = |x: f32, z: f32| -> (i32, i32) {
            let px = ((x - min_x) / span_x * (size - 1) as f32) as i32;
            let py = ((z - min_z) / span_z * (size - 1) as f32) as i32;
            (px.clamp(0, size as i32 - 1), py.clamp(0, size as i32 - 1))
        };
        let mut values = vec![0u8; size * size];
        let line = |a: (i32, i32), b: (i32, i32), shade: u8, values: &mut Vec<u8>| {
            let steps = (a.0 - b.0).abs().max((a.1 - b.1).abs()).max(1);
            for step in 0..=steps {
                let t = step as f32 / steps as f32;
                let x = (a.0 as f32 + (b.0 - a.0) as f32 * t).round() as i32;
                let y = (a.1 as f32 + (b.1 - a.1) as f32 * t).round() as i32;
                if x >= 0 && y >= 0 && (x as usize) < size && (y as usize) < size {
                    let at = y as usize * size + x as usize;
                    values[at] = values[at].max(shade);
                }
            }
        };
        for quad in &stage.quads {
            let points: Vec<(i32, i32)> = quad
                .corners
                .iter()
                .map(|corner| project(corner[0], corner[2]))
                .collect();
            for index in 0..4 {
                line(points[index], points[(index + 1) % 4], 190, &mut values);
            }
        }
        for plate in &stage.plates {
            let (px, py) = project(plate.position[0], plate.position[2]);
            for (dx, dy) in [(0i32, 0i32), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                let x = px + dx;
                let y = py + dy;
                if x >= 0 && y >= 0 && (x as usize) < size && (y as usize) < size {
                    values[y as usize * size + x as usize] = 255;
                }
            }
        }
        out.bitmap = Some(GrayMap {
            width: size as u32,
            height: size as u32,
            values,
            legend: format!(
                "top view (x = X {min_x:.0}..{max_x:.0}, y = Z {min_z:.0}..{max_z:.0}); grey = terrain outline, white = plate centre"
            ),
        });
    }
    out
}

fn acf(bytes: &[u8]) -> Projection {
    use super::document::utf;
    let mut out = Projection::new("ACF config tables");
    let tables = utf::scan_tables(bytes);
    out.note = Some(format!("{} @UTF tables", tables.len()));
    for (index, table) in tables.iter().take(6).enumerate() {
        let columns: Vec<String> = table
            .columns
            .iter()
            .take(12)
            .map(|column| column.name.clone())
            .collect();
        let mut rows = Vec::new();
        for row in 0..table.row_count.min(40) {
            let mut at = table.rows_at() + row * table.row_width;
            let mut cells = Vec::new();
            for column in table.columns.iter().take(12) {
                if !column.row_storage {
                    cells.push(String::new());
                    continue;
                }
                cells.push(utf::cell_text(bytes, table, at, column));
                at += column.size;
            }
            rows.push(cells);
        }
        out.sections.push(Section::new(
            &format!("table {index} @{:#x}", table.at),
            &columns.iter().map(String::as_str).collect::<Vec<_>>(),
            rows,
            table.row_count as u64,
        ));
    }
    out
}

fn ufb(bytes: &[u8]) -> Projection {
    super::field::view(bytes, None, 1500)
}

fn ucb(bytes: &[u8]) -> Projection {
    const RECORD: usize = 0x20;
    const MAX_SIDE: usize = 1200;
    const MAX_AREAS: usize = 16;
    const MAX_CLASSES: usize = 12;

    let cells = bytes.len() / RECORD;
    let mut out = Projection::new("UCB · world map cells");

    if cells == 0 {
        out.note = Some("no complete 0x20-byte cell".to_string());
        return out;
    }

    let mut width = 0usize;
    let mut height = 0usize;
    let mut outside = 0u64;
    let mut classes: BTreeMap<u32, u64> = BTreeMap::new();
    for index in 0..cells {
        let at = index * RECORD;
        width = width.max(rdr::u32_at(bytes, at).unwrap_or(0) as usize);
        height = height.max(rdr::u32_at(bytes, at + 4).unwrap_or(0) as usize);
        let word = rdr::u32_at(bytes, at + 8).unwrap_or(0);
        if word == 0 {
            outside += 1;
        } else {
            *classes.entry(word >> 22).or_default() += 1;
        }
    }
    let grid_width = width + 1;
    let grid_height = height + 1;

    let step = grid_width.max(grid_height).div_ceil(MAX_SIDE).max(1);
    let out_width = grid_width.div_ceil(step);
    let out_height = grid_height.div_ceil(step);
    let mut rgba = vec![0u8; out_width * out_height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[10, 12, 18, 255]);
    }

    struct Area {
        cells: u64,
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
    }

    let mut areas: BTreeMap<u32, Area> = BTreeMap::new();
    for index in 0..cells {
        let at = index * RECORD;
        let x = rdr::u32_at(bytes, at).unwrap_or(0) as usize;
        let y = rdr::u32_at(bytes, at + 4).unwrap_or(0) as usize;
        let word = rdr::u32_at(bytes, at + 8).unwrap_or(0);
        if word == 0 {
            continue;
        }

        let area = word & 0xffff;
        let px = (x / step).min(out_width - 1);
        let py = (y / step).min(out_height - 1);
        let mut color = hsv(area_hue(area), 0.62, 0.92);

        let left = x > 0 && rdr::u32_at(bytes, at - RECORD + 8).unwrap_or(0) & 0xffff != area;
        let above =
            y > 0 && rdr::u32_at(bytes, at - grid_width * RECORD + 8).unwrap_or(0) & 0xffff != area;
        let edge = left || above;
        if edge {
            color = hsv(area_hue(area), 0.35, 0.30);
        }
        put(&mut rgba, (py * out_width + px) * 4, color);

        let entry = areas.entry(area).or_insert(Area {
            cells: 0,
            x0: x,
            y0: y,
            x1: x,
            y1: y,
        });
        entry.cells += 1;
        entry.x0 = entry.x0.min(x);
        entry.y0 = entry.y0.min(y);
        entry.x1 = entry.x1.max(x);
        entry.y1 = entry.y1.max(y);
    }

    out.note = Some(format!(
        "{cells} cells laid out as {grid_width}×{grid_height} (x right, y down) · {} areas · {outside} cells with no region. Cell x runs along world x from -42480 at 32 units per cell, cell y along world z from -18416; the id is the mask index in Field.ufb, and re-painting those masks in +0x34 order reproduces this whole grid. The file stores no pixels: the map screen art ships as GUI/Worldmap.mbs + GUI/Worldmap.ftx, and each area's field background as FieldBG/AC*.mpd + AC*.ftx.",
        areas.len()
    ));

    let grid = vec![
        vec!["cells".to_string(), cells.to_string()],
        vec!["width (x)".to_string(), grid_width.to_string()],
        vec!["height (y)".to_string(), grid_height.to_string()],
        vec!["cell size".to_string(), "0x20 bytes".to_string()],
        vec!["areas".to_string(), areas.len().to_string()],
        vec!["all-zero cells".to_string(), outside.to_string()],
        vec![
            "draw step".to_string(),
            format!("1 px = {step}×{step} cells"),
        ],
    ];
    out.sections
        .push(Section::new("Grid", &["item", "value"], grid, 7));

    let layout = vec![
        vec![
            "+0x00".to_string(),
            "u32 x".to_string(),
            "column, 0..width-1".to_string(),
        ],
        vec![
            "+0x04".to_string(),
            "u32 y".to_string(),
            "row, 0..height-1".to_string(),
        ],
        vec![
            "+0x08".to_string(),
            "u32 packed".to_string(),
            "0 = outside the map; otherwise bits 0..15 = mask index in Field.ufb, bit 16..21 = 1 on every in-map cell, bits 22..31 = terrain class".to_string(),
        ],
        vec![
            "+0x0c".to_string(),
            "u32".to_string(),
            "0xff000000 in every cell of this file".to_string(),
        ],
        vec![
            "+0x10..0x20".to_string(),
            "16 bytes".to_string(),
            "zero in every cell of this file".to_string(),
        ],
    ];
    out.sections.push(Section::new(
        "Cell layout",
        &["offset", "field", "meaning"],
        layout,
        5,
    ));

    let mut ranked: Vec<(u32, &Area)> = areas.iter().map(|(id, area)| (*id, area)).collect();
    ranked.sort_by(|left, right| right.1.cells.cmp(&left.1.cells));
    let rows: Vec<Vec<String>> = ranked
        .iter()
        .take(MAX_AREAS)
        .map(|(id, area)| {
            vec![
                id.to_string(),
                area.cells.to_string(),
                format!("({}, {})..({}, {})", area.x0, area.y0, area.x1, area.y1),
                format!("{}×{}", area.x1 - area.x0 + 1, area.y1 - area.y0 + 1),
            ]
        })
        .collect();
    out.sections.push(Section::new(
        "Areas (by cell count)",
        &["area", "cells", "bbox", "size"],
        rows,
        areas.len() as u64,
    ));

    let mut classes_ranked: Vec<(u32, u64)> = classes.into_iter().collect();
    classes_ranked.sort_by(|left, right| right.1.cmp(&left.1));
    let total: u64 = classes_ranked.iter().map(|(_, count)| count).sum();
    let class_rows: Vec<Vec<String>> = classes_ranked
        .iter()
        .take(MAX_CLASSES)
        .map(|(id, count)| {
            vec![
                id.to_string(),
                count.to_string(),
                format!("{:.1}%", 100.0 * *count as f64 / total.max(1) as f64),
            ]
        })
        .collect();
    out.sections.push(Section::new(
        "Terrain classes",
        &["class", "cells", "share"],
        class_rows,
        classes_ranked.len() as u64,
    ));

    out.color = Some(ColorMap {
        width: out_width as u32,
        height: out_height as u32,
        rgba,
        legend: format!(
            "Canvas {out_width}×{out_height} px = the {grid_width}×{grid_height} world grid (x right, y down, 1 px = {step}×{step} cells). Hue = the area each cell belongs to, i.e. the low 16 bits of `+0x08`; that number is the mask index inside Field.ufb, so one colour = one field area. Painting every Field.ufb mask cell at 32 world units per cell, in ascending +0x34 order, reproduces this grid cell for cell. Dark seams = where the area changes between neighbouring cells. Terrain class is not drawn (see the table); the {outside} cells with no region are left near-black.",
        ),
    });

    out
}

fn scb(bytes: &[u8]) -> Projection {
    let mut out = Projection::new("SCB compendium script");
    let count = rdr::u32_at(bytes, 0).unwrap_or(0) as usize;
    let mut tokens: Vec<String> = Vec::new();
    let mut cursor = 4;
    for _ in 0..count.min(200_000) {
        let Some(len) = rdr::u32_at(bytes, cursor) else {
            break;
        };
        let len = len as usize;
        if len == 0 || cursor + 4 + len > bytes.len() {
            break;
        }
        let raw = &bytes[cursor + 4..cursor + 4 + len];
        let text = raw.split(|value| *value == 0).next().unwrap_or(&[]);
        tokens.push(String::from_utf8_lossy(text).to_string());
        cursor += 4 + len;
    }
    out.note = Some(format!("{count} tokens; braces drive the indentation"));

    let mut lines: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut line = String::new();
    for token in &tokens {
        match token.as_str() {
            "{" => {
                if !line.is_empty() {
                    lines.push(format!("{}{} {{", "  ".repeat(depth), line.trim()));
                    line.clear();
                } else {
                    lines.push(format!("{}{{", "  ".repeat(depth)));
                }
                depth += 1;
            }
            "}" => {
                if !line.is_empty() {
                    lines.push(format!("{}{}", "  ".repeat(depth), line.trim()));
                    line.clear();
                }
                depth = depth.saturating_sub(1);
                lines.push(format!("{}}}", "  ".repeat(depth)));
            }
            ";" => {
                if !line.is_empty() {
                    lines.push(format!("{}{};", "  ".repeat(depth), line.trim()));
                    line.clear();
                }
            }
            other => {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(other);
            }
        }
    }
    if !line.is_empty() {
        lines.push(format!("{}{}", "  ".repeat(depth), line.trim()));
    }
    out.text = Some(lines);
    out
}

fn inc(bytes: &[u8]) -> Projection {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<String> = text.lines().take(20_000).map(str::to_string).collect();
    let mut out = Projection::new("INC enum table");
    out.note = Some(format!("{} lines", text.lines().count()));
    out.text = Some(lines);
    out
}

fn mpd(bytes: &[u8]) -> Projection {
    let count = rdr::u32_at(bytes, 4).unwrap_or(0);
    let mut out = Projection::new("MPD quad records");
    out.note = Some(format!("header 0x20 + {count} × 0xA0"));
    let rows = record_rows(bytes, 0x20, 0xa0, count as usize, 500);
    out.sections.push(Section::new(
        "Records",
        &["#", "u32 ×6", "f32 ×6"],
        rows,
        count as u64,
    ));
    out
}

fn ctb(bytes: &[u8]) -> Projection {
    let mut out = Projection::new("CTB battle icon animation");
    out.note = Some("0x58 header + 7 sections (clip records + 6 fixed-size blocks)".to_string());
    let strides = [0x58usize, 0x20, 0x0c, 0x18, 0x04, 0x01, 0x0c];
    let mut summary = Vec::new();
    let mut cursor = 0x58;
    for (index, stride) in strides.iter().enumerate() {
        if cursor + stride > bytes.len() {
            break;
        }
        let mut count = 0usize;
        while cursor + stride <= bytes.len() && count < 4096 {
            cursor += stride;
            count += 1;
            if index == 0 {
                break;
            }
        }
        summary.push(vec![
            index.to_string(),
            format!("{stride:#x}"),
            count.to_string(),
            format!("{:#x}", cursor),
        ]);
    }
    out.sections.push(Section::new(
        "Sections",
        &["#", "stride", "count", "end offset"],
        summary,
        7,
    ));
    let rows = record_rows(bytes, 0x58, 0x58, 200, 8);
    out.sections
        .push(Section::new("Sample", &["#", "u32 ×4", "f32 ×4"], rows, 32));
    out
}

fn record_rows(
    bytes: &[u8],
    start: usize,
    stride: usize,
    count: usize,
    limit: usize,
) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for index in 0..count.min(limit) {
        let at = start + index * stride;
        if at + stride > bytes.len() {
            break;
        }
        let mut ints = String::new();
        for word in 0..6 {
            ints.push_str(&format!(
                "{:>10} ",
                rdr::u32_at(bytes, at + word * 4).unwrap_or(0)
            ));
        }
        let mut floats = String::new();
        for word in 0..6 {
            let value = rdr::f32_at(bytes, at + word * 4).unwrap_or(0.0);
            if value.is_finite() && value.abs() < 1.0e6 {
                floats.push_str(&format!("{value:>10.3} "));
            } else {
                floats.push_str(&format!("{:>10} ", "—"));
            }
        }
        rows.push(vec![
            index.to_string(),
            ints.trim_end().to_string(),
            floats.trim_end().to_string(),
        ]);
    }
    rows
}
