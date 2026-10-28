//! 内容视图投影。
//!
//! 文档树回答「文件里有什么字段」，这一层回答「文件里是什么东西」：
//! 调试库是几千条命令、语音表是一张标志位图、节点网络是一簇位模式、图鉴是一段可读源码。
//!
//! 这里只产原始数据（表格 / 灰度位图 / 折线 / 文本），PNG 编码在 `vw-media`。

use super::doc::rdr;

/// 掩码位图横向分段上限：条数再多也只折成这么多列。
const MAX_BIT_COLUMNS: usize = 768;

/// 一张表。
pub struct Section {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// 实际总行数（`rows` 可能被截断）。
    pub total: u64,
}

impl Section {
    fn new(title: &str, columns: &[&str], rows: Vec<Vec<String>>, total: u64) -> Self {
        Self {
            title: title.to_string(),
            columns: columns.iter().map(|value| value.to_string()).collect(),
            rows,
            total,
        }
    }
}

/// 把「每行 width 位的掩码流」折成横向位图：**宽 = 条目分段，高 = 位序号**。
///
/// 条目多的时候竖着排会变成一条细线，横过来才看得出「哪一段出现了哪些位」。
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

/// 灰度位图（0/255），一行一个样本。
pub struct GrayMap {
    pub width: u32,
    pub height: u32,
    pub values: Vec<u8>,
    pub legend: String,
}

/// 单条折线。
pub struct Series {
    pub label: String,
    pub points: Vec<f32>,
    pub note: Option<String>,
}

/// 投影结果。
pub struct Projection {
    pub title: String,
    pub note: Option<String>,
    pub sections: Vec<Section>,
    pub bitmap: Option<GrayMap>,
    pub series: Option<Series>,
    pub text: Option<Vec<String>>,
}

impl Projection {
    fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            note: None,
            sections: Vec::new(),
            bitmap: None,
            series: None,
            text: None,
        }
    }
}

/// 按扩展名投影；没有内容视图的格式返回 `None`。
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

/// 位图行优先：每个样本占 `width` 个字节（值是 0/255）。
fn push_bits(values: &mut Vec<u8>, word: u64, width: u32) {
    for bit in 0..width {
        let on = (word >> (width - 1 - bit)) & 1 == 1;
        values.push(if on { 255 } else { 0 });
    }
}

// ─── DRG：调试参数库 ───────────────────────────────

fn drg(bytes: &[u8]) -> Projection {
    let db_count = rdr::u32_at(bytes, 4).unwrap_or(0) as usize;
    let cmd_count = rdr::u32_at(bytes, 8).unwrap_or(0) as usize;
    let mut out = Projection::new("DbReg 调试参数库");
    out.note = Some(format!("{db_count} 个数据库 / {cmd_count} 条命令"));

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
            "数据库",
            &["#", "原始路径", "名称", "标签", "u32 ×4"],
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
            "命令",
            &["#", "命令名", "说明"],
            rows,
            cmd_count as u64,
        ));
    } else if bytes.len() > 0x38 {
        // DbReg.drg 没有记录表，体是一串 f32（6546 个）。
        let count = (bytes.len() - 0x38) / 4;
        let points: Vec<f32> = (0..count.min(4096))
            .filter_map(|index| rdr::f32_at(bytes, 0x38 + index * 4))
            .filter(|value| value.is_finite())
            .collect();
        out.note = Some(format!("没有数据库/命令记录；体是 {count} 个 f32"));
        out.series = Some(Series {
            label: "f32 值".to_string(),
            points,
            note: Some("按文件顺序".to_string()),
        });
    }
    out
}

// ─── VS2：语音发声表 ───────────────────────────────

fn vs2(bytes: &[u8]) -> Projection {
    let count = rdr::u32_at(bytes, 0).unwrap_or(0) as usize;
    let mut out = Projection::new("VoiceSpeak 语音表");
    let ids_at = 0x10;
    let mask_at = ids_at + count * 4;
    let expected = mask_at + count * 8;
    out.note = Some(format!(
        "{count} 条；16 + {count}×4 + {count}×8 = {expected}（文件 {}）",
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
        "条目",
        &["#", "语音 id", "标志位", "置位数"],
        rows,
        count as u64,
    ));
    let (values, columns, bits) = fold_bits(&values, 64, MAX_BIT_COLUMNS);
    out.bitmap = Some(GrayMap {
        width: columns as u32,
        height: bits as u32,
        values,
        legend: format!(
            "横轴 = {count} 条折成的 {columns} 段序号，纵轴 = 标志位 63（上）..0（下）；白 = 该段内出现过"
        ),
    });
    out
}

// ─── NND：节点网络 ─────────────────────────────────

fn nnd(bytes: &[u8]) -> Projection {
    let words = rdr::u32_at(bytes, 4).unwrap_or(0) as usize;
    let units = (words / 3).min(200_000);
    let mut out = Projection::new("NodeNetwork 节点网络");
    out.note = Some(format!(
        "{units} 组三字；第三字按 32 位展开，第二字是数值序列"
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
        .push(Section::new("头部", &["偏移", "值"], head, 6));

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
            "横轴 = {units} 组折成的 {columns} 段序号，纵轴 = 第三字 31（上）..0（下）；白 = 该段内出现过"
        ),
    });
    out.series = Some(Series {
        label: "第二字".to_string(),
        points: series,
        note: Some("按组序号；取值 0～128 的浮点".to_string()),
    });

    let names = nnd_tail_names(bytes, 0x18 + words * 4);
    if !names.is_empty() {
        let rows: Vec<Vec<String>> = names
            .iter()
            .enumerate()
            .map(|(index, name)| vec![index.to_string(), name.clone()])
            .collect();
        let total = rows.len() as u64;
        out.sections
            .push(Section::new("尾部地名", &["#", "名字"], rows, total));
    }
    out
}

/// 尾部 64 字节名字槽（前导区可能是几百个索引字，所以扫 8 KB 窗口）。
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

// ─── FSB：场景包（13 侧动画舞台）───────────────────

fn fsb(bytes: &[u8]) -> Projection {
    let mut out = Projection::new("FSB 场景包");

    // 块链：`magic(4) + u32 块长`，首尾相接走到 EOF。
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
        "{} 个块，走到 {pos} 字节（共 {}）",
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
                format!("版本 {version:#04x} / 常量 {scale:.3}")
            }
            b"RESL" => {
                // u32 count，之后每条 = u32 长度（含补零）+ 名字。
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
                format!("{count} 个资源")
            }
            b"CTLG" => {
                // u32 count，之后每条 4 个 u32。
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
                format!("{count} 条")
            }
            b"SCNL" => {
                let count = rdr::u32_at(bytes, body).unwrap_or(0);
                let stride = rdr::u32_at(bytes, body + 4).unwrap_or(0);
                scene_note = format!(
                    "{} 段 × {stride} 字节；首段名 {}",
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
        "块",
        &["块", "偏移", "大小", "摘要"],
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
        out.sections
            .push(Section::new("外部资源", &["#", "文件名"], rows, total));
    }

    if !catalog.is_empty() {
        let total = catalog.len() as u64;
        out.sections.push(Section::new(
            "目录",
            &["#", "字段 1", "字段 2", "字段 3", "索引"],
            catalog,
            total,
        ));
    }

    if !scene_note.is_empty() {
        out.sections.push(Section::new(
            "场景",
            &["项", "值"],
            vec![vec!["说明".to_string(), scene_note]],
            1,
        ));
    }
    out
}

// ─── OEB：13 侧的 NPC 摆位 ─────────────────────────

fn oeb(bytes: &[u8]) -> Projection {
    use super::doc::oeb as doc;
    let slots = doc::slots(bytes);
    let resources = doc::resources(bytes);
    let mut out = Projection::new("OEB 角色摆位");
    out.note = Some(format!(
        "{} 个站位 / {} 个外部资源（.mbs 与 .ftx 成对）",
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
            "站位",
            &["名字", "角色", "X", "Y", "Z", "缩放"],
            rows,
            total,
        ));

        // 俯视散点：横轴 = X，纵轴 = Y（与 .osb 舞台同一套单位）。
        let min_x = slots.iter().map(|s| s.position[0]).fold(f32::MAX, f32::min);
        let max_x = slots.iter().map(|s| s.position[0]).fold(f32::MIN, f32::max);
        let min_y = slots.iter().map(|s| s.position[1]).fold(f32::MAX, f32::min);
        let max_y = slots.iter().map(|s| s.position[1]).fold(f32::MIN, f32::max);
        let size = 256usize;
        let mut values = vec![0u8; size * size];
        for slot in &slots {
            let x = scale(slot.position[0], min_x, max_x, size);
            let y = scale(slot.position[1], min_y, max_y, size);
            // 点画成十字，单像素在缩略图上会看不见。
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
                "站位俯视图：横轴 X（{min_x:.0}~{max_x:.0}），纵轴 Y（{min_y:.0}~{max_y:.0}），白点 = 一个角色"
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
            .push(Section::new("资源", &["#", "路径"], rows, total));
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

// ─── ORB：13 侧的轮次镜头 ──────────────────────────

fn orb(bytes: &[u8]) -> Projection {
    use super::doc::orb as doc;
    let cameras = doc::cameras(bytes);
    let resources = doc::resources(bytes);
    let mut out = Projection::new("ORB 轮次镜头");
    out.note = Some(format!(
        "{} 次镜头 / {} 个资源路径",
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
            "镜头",
            &["#", "舞台", "序号", "X", "Y", "Z"],
            rows,
            total,
        ));
    }

    if !resources.is_empty() {
        let rows: Vec<Vec<String>> = resources
            .iter()
            .enumerate()
            .map(|(index, path)| {
                // 资源池里存的是完整工程路径，只显示 Data\ 之后的部分。
                let short = path
                    .rsplit_once("Data\\")
                    .map(|(_, rest)| rest.to_string())
                    .unwrap_or_else(|| path.clone());
                vec![index.to_string(), short, path.clone()]
            })
            .collect();
        let total = rows.len() as u64;
        out.sections.push(Section::new(
            "资源",
            &["#", "相对路径", "原始路径"],
            rows,
            total,
        ));
    }
    out
}

// ─── BST：13 侧的战斗关卡 ──────────────────────────

fn bst(bytes: &[u8]) -> Projection {
    use super::doc::bst as doc;
    let names = doc::resources(bytes);
    let idents = doc::identifiers(bytes);
    let mut out = Projection::new("BST 战斗关卡");
    if bytes.len() < 0x1b {
        out.note = Some("文件不足 0x1b 字节".to_string());
        return out;
    }
    let records = rdr::u32_at(bytes, 0x10).unwrap_or(0);
    out.note = Some(format!(
        "{records} 条记录 / {} 个资源 / {} 个标识符；条目结构未解",
        names.len(),
        idents.len()
    ));

    let head = vec![
        vec!["魔数".to_string(), rdr::ascii(&bytes[..4])],
        vec!["条目数".to_string(), records.to_string()],
        vec![
            "名字区".to_string(),
            format!(
                "0x{:x} 起，{} 字节",
                0x10 + bytes[0x14] as usize,
                bytes[0x16]
            ),
        ],
        vec!["变体号".to_string(), format!("0x{:x}", bytes[0x17])],
        vec!["文件长".to_string(), bytes.len().to_string()],
    ];
    out.sections
        .push(Section::new("头部", &["项", "值"], head, 5));

    if !names.is_empty() {
        let rows: Vec<Vec<String>> = names
            .iter()
            .enumerate()
            .map(|(index, name)| vec![index.to_string(), name.clone()])
            .collect();
        let total = rows.len() as u64;
        out.sections
            .push(Section::new("资源", &["#", "文件"], rows, total));
    }

    if !idents.is_empty() {
        let rows: Vec<Vec<String>> = idents
            .iter()
            .enumerate()
            .map(|(index, ident)| vec![index.to_string(), ident.clone()])
            .collect();
        let total = rows.len() as u64;
        out.sections
            .push(Section::new("标识符", &["#", "名字"], rows, total));
    }
    out
}

// ─── ACF：CRI 配置表 ───────────────────────────────

fn acf(bytes: &[u8]) -> Projection {
    use super::doc::utf;
    let mut out = Projection::new("ACF 配置表");
    let tables = utf::scan_tables(bytes);
    out.note = Some(format!("{} 张 @UTF 表", tables.len()));
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
            &format!("表 {index} @{:#x}", table.at),
            &columns.iter().map(String::as_str).collect::<Vec<_>>(),
            rows,
            table.row_count as u64,
        ));
    }
    out
}

// ─── UFB：大地图图层装配 ───────────────────────────

fn ufb(bytes: &[u8]) -> Projection {
    const HEAD: usize = 0x88;
    let mut out = Projection::new("UFB 大地图图层");
    if bytes.len() < HEAD {
        out.note = Some("文件不足 0x88 字节".to_string());
        return out;
    }
    let count_a = rdr::u32_at(bytes, 0x40).unwrap_or(0) as usize;
    let grid_count = rdr::u32_at(bytes, 0x54).unwrap_or(0) as usize;

    let mut head = Vec::new();
    for index in 0..12 {
        let at = 0x40 + index * 4;
        head.push(vec![
            format!("+{at:#04x}"),
            rdr::u32_at(bytes, at).unwrap_or(0).to_string(),
        ]);
    }
    out.sections
        .push(Section::new("头部计数", &["偏移", "值"], head, 12));
    out.note = Some(format!("{count_a} 条记录 / {grid_count} 个网格"));

    // 记录区：0x24 头 + 层 + 扩展块 + 组。
    let mut cursor = HEAD;
    let mut layers_total = 0usize;
    for _ in 0..count_a {
        if cursor + 0x24 > bytes.len() {
            break;
        }
        let layers = rdr::u16_at(bytes, cursor + 0x20).unwrap_or(0) as usize;
        let ext = rdr::u8_at(bytes, cursor + 0x22).unwrap_or(0) as usize;
        let groups = rdr::u8_at(bytes, cursor + 0x23).unwrap_or(0) as usize;
        layers_total += layers;
        cursor += 0x24 + layers * 0x40 + ext * 0x90;
        for _ in 0..groups {
            if cursor + 4 > bytes.len() {
                break;
            }
            let entries = rdr::u8_at(bytes, cursor + 3).unwrap_or(0) as usize;
            cursor += 4 + entries * 0xF0;
        }
    }
    out.sections.push(Section::new(
        "记录区",
        &["项", "值"],
        vec![
            vec!["记录数".to_string(), count_a.to_string()],
            vec!["层总数".to_string(), layers_total.to_string()],
            vec!["记录区结束".to_string(), format!("{cursor:#x}")],
        ],
        3,
    ));

    // 网格区：0x3c 头 + w*h*4。
    let mut grids = Vec::new();
    let mut start = cursor;
    let mut first: Option<(usize, usize, usize, usize, usize)> = None;
    for index in 0..grid_count.min(64) {
        if start + 0x3c > bytes.len() {
            break;
        }
        let x0 = rdr::i32_at(bytes, start + 0x24).unwrap_or(0);
        let y0 = rdr::i32_at(bytes, start + 0x28).unwrap_or(0);
        let x1 = rdr::i32_at(bytes, start + 0x2c).unwrap_or(0);
        let y1 = rdr::i32_at(bytes, start + 0x30).unwrap_or(0);
        let width = (x1 - x0 + 1).max(0) as usize;
        let height = (y1 - y0 + 1).max(0) as usize;
        let data = start + 0x3c;
        grids.push(vec![
            index.to_string(),
            format!("({x0}, {y0})"),
            format!("({x1}, {y1})"),
            format!("{width}×{height}"),
            format!("{data:#x}"),
        ]);
        if first.is_none() && width > 0 && height > 0 {
            first = Some((data, width, height, x0 as usize, y0 as usize));
        }
        start = data + width * height * 4;
    }
    let total = grids.len() as u64;
    out.sections.push(Section::new(
        "网格",
        &["#", "起点", "终点", "尺寸", "数据偏移"],
        grids,
        total,
    ));

    // 首个网格画热力图：取 u32 低 8 位，超过 512 就抽样。
    if let Some((data, width, height, _, _)) = first {
        let step_x = width.div_ceil(512).max(1);
        let step_y = height.div_ceil(512).max(1);
        let out_width = width.div_ceil(step_x) as u32;
        let out_height = height.div_ceil(step_y) as u32;
        let mut raw = Vec::with_capacity((out_width as usize) * (out_height as usize));
        let mut low = u32::MAX;
        let mut high = 0u32;
        for row in (0..height).step_by(step_y) {
            for column in (0..width).step_by(step_x) {
                let at = data + (row * width + column) * 4;
                let word = rdr::u32_at(bytes, at).unwrap_or(0);
                low = low.min(word);
                high = high.max(word);
                raw.push(word);
            }
        }
        // 值域拉满；全域同值时退化成按低 8 位看位模式。
        let span = high.saturating_sub(low);
        let values: Vec<u8> = raw
            .iter()
            .map(|word| {
                if span == 0 {
                    (*word & 0xff) as u8
                } else {
                    (((word - low) as f64 / span as f64) * 255.0) as u8
                }
            })
            .collect();
        out.bitmap = Some(GrayMap {
            width: out_width,
            height: out_height,
            values,
            legend: format!(
                "首个网格 {width}×{height}，取样 1/{step_x}×1/{step_y}；灰度 = u32 归一化（{low:#x}～{high:#x}）"
            ),
        });
    }
    out
}

// ─── UCB：大地图单元 ───────────────────────────────

fn ucb(bytes: &[u8]) -> Projection {
    const RECORD: usize = 0x20;
    const GROUP_SIZE: usize = 2199;
    let records = bytes.len() / RECORD;
    let mut out = Projection::new("UCB 大地图单元");
    out.note = Some(format!(
        "{records} 条 × 0x20；{} 组 × {GROUP_SIZE} 条",
        records / GROUP_SIZE
    ));

    let head = vec![
        vec!["记录数".to_string(), records.to_string()],
        vec!["组数".to_string(), (records / GROUP_SIZE).to_string()],
        vec!["每组条数".to_string(), GROUP_SIZE.to_string()],
        vec!["文件长".to_string(), bytes.len().to_string()],
    ];
    out.sections
        .push(Section::new("规模", &["项", "值"], head, 4));

    let mut rows = Vec::new();
    for index in 0..records.min(64) {
        let at = index * RECORD;
        rows.push(vec![
            index.to_string(),
            rdr::u32_at(bytes, at).unwrap_or(0).to_string(),
            rdr::u32_at(bytes, at + 4).unwrap_or(0).to_string(),
            format!("{:#010x}", rdr::u32_at(bytes, at + 8).unwrap_or(0)),
            format!("{:#010x}", rdr::u32_at(bytes, at + 0x0c).unwrap_or(0)),
        ]);
    }
    out.sections.push(Section::new(
        "前 64 条",
        &["#", "区内索引", "组号", "+0x08", "+0x0c"],
        rows,
        64,
    ));

    // 地图：横轴 = 组内下标（2199），纵轴 = 组号（1311），灰度 = `+0x08` 的高位类别。
    let groups = records / GROUP_SIZE;
    if groups > 0 {
        let step_x = GROUP_SIZE.div_ceil(512).max(1);
        let step_y = groups.div_ceil(512).max(1);
        let out_width = GROUP_SIZE.div_ceil(step_x);
        let out_height = groups.div_ceil(step_y);
        let mut classes: Vec<u8> = Vec::with_capacity(out_width * out_height);
        let mut high = 0u32;
        for group in (0..groups).step_by(step_y) {
            for local in (0..GROUP_SIZE).step_by(step_x) {
                let at = (group * GROUP_SIZE + local) * RECORD + 8;
                let kind = rdr::u32_at(bytes, at).unwrap_or(0) >> 22 & 0xff;
                high = high.max(kind);
                classes.push(kind as u8);
            }
        }
        // 类别是小整数（0/1/2/5/6/15/26/36…），线性拉满会把 1/2 压成黑，
        // 开方后低类别也能看出轮廓：0 仍是黑（世界外），1 → 40，36 → 245。
        let span = high.max(1) as f64;
        let values = classes
            .iter()
            .map(|kind| (((*kind as f64 / span).sqrt()) * 255.0) as u8)
            .collect();
        out.bitmap = Some(GrayMap {
            width: out_width as u32,
            height: out_height as u32,
            values,
            legend: format!(
                "横轴 = 组内下标 {GROUP_SIZE}，纵轴 = 组号 {groups}；灰度 = `+0x08` 第 22 位起的类别字节（0～{high}）"
            ),
        });
    }
    out
}

// ─── SCB：图鉴脚本 ─────────────────────────────────

fn scb(bytes: &[u8]) -> Projection {
    let mut out = Projection::new("SCB 图鉴脚本");
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
    out.note = Some(format!("{count} 个令牌；按大括号缩进还原成源码"));

    // 令牌流 → 可读文本：`{` 换行缩进，`;` 换行。
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

// ─── INC：开发期枚举表 ─────────────────────────────

fn inc(bytes: &[u8]) -> Projection {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<String> = text.lines().take(20_000).map(str::to_string).collect();
    let mut out = Projection::new("INC 枚举表");
    out.note = Some(format!("{} 行", text.lines().count()));
    out.text = Some(lines);
    out
}

// ─── MPD / CTB：记录表 ─────────────────────────────

fn mpd(bytes: &[u8]) -> Projection {
    let count = rdr::u32_at(bytes, 4).unwrap_or(0);
    let mut out = Projection::new("MPD 四边形记录");
    out.note = Some(format!("头 0x20 + {count} 条 × 0xA0"));
    let rows = record_rows(bytes, 0x20, 0xa0, count as usize, 500);
    out.sections.push(Section::new(
        "记录",
        &["#", "u32 ×6", "f32 ×6"],
        rows,
        count as u64,
    ));
    out
}

fn ctb(bytes: &[u8]) -> Projection {
    let mut out = Projection::new("CTB 战斗图标动画");
    out.note = Some("0x58 头 + 7 节（clip 记录 + 6 段定长）".to_string());
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
        "节",
        &["#", "步长", "条数", "结束偏移"],
        summary,
        7,
    ));
    let rows = record_rows(bytes, 0x58, 0x58, 200, 8);
    out.sections
        .push(Section::new("样本", &["#", "u32 ×4", "f32 ×4"], rows, 32));
    out
}

/// 通用记录表：前 6 个 u32 + 前 6 个 f32（按 stride 走）。
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
            // 同一个槽位在别的记录里可能是整数或字符串，超出合理范围就当作「不是浮点」。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drg_lists_databases_and_commands() {
        let mut bytes = vec![0u8; 0x38 + 2 * 0x90];
        bytes[..4].copy_from_slice(b"DRG\0");
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&0u32.to_le_bytes());
        bytes[0x38..0x38 + 5].copy_from_slice(b"DEBUG");
        let projection = project(&bytes, "drg").expect("drg 应当有内容视图");
        assert!(projection.sections[0].columns.contains(&"名称".to_string()));
        assert_eq!(projection.sections[0].rows.len(), 2);
    }

    #[test]
    fn vs2_expands_flag_bitmap() {
        let count = 3usize;
        let mut bytes = vec![0u8; 0x10 + count * 4 + count * 8];
        bytes[0..4].copy_from_slice(&(count as u32).to_le_bytes());
        for index in 0..count {
            let at = 0x10 + index * 4;
            bytes[at..at + 4].copy_from_slice(&(1000u32 + index as u32).to_le_bytes());
        }
        bytes[0x10 + count * 4..0x10 + count * 4 + 8].copy_from_slice(&1u64.to_le_bytes());
        let projection = project(&bytes, "vs2").expect("vs2 应当有内容视图");
        let bitmap = projection.bitmap.expect("应当有位图");
        // 横轴是条目（折成分段），纵轴是位序号；三条都不到一列的上限，所以宽 = 3、高 = 64。
        assert_eq!(bitmap.width, count as u32);
        assert_eq!(bitmap.height, 64);
        // 只有第 0 条的 bit 0 置位；位 63 在纵轴最上面。
        assert_eq!(bitmap.values[63 * count], 255);
        assert_eq!(bitmap.values[63 * count + 1], 0);
    }

    #[test]
    fn scb_renders_readable_source() {
        let mut bytes = Vec::new();
        let tokens = ["ROOT", "{", "@PAGE", "(", "TITLE", ")", "{", "}", "}"];
        bytes.extend_from_slice(&(tokens.len() as u32).to_le_bytes());
        for token in tokens {
            let raw = token.as_bytes();
            bytes.extend_from_slice(&((raw.len() + 1) as u32).to_le_bytes());
            bytes.extend_from_slice(raw);
            bytes.push(0);
        }
        let projection = project(&bytes, "scb").expect("scb 应当有内容视图");
        let lines = projection.text.expect("应当是文本视图");
        assert!(lines.iter().any(|line| line.contains("@PAGE")));
        assert_eq!(lines.first().map(String::as_str), Some("ROOT {"));
    }
}
