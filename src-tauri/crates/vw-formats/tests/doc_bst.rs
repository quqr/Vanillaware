use vw_formats::doc::bst::*;

fn push_text(out: &mut Vec<u8>, value: &str) {
    out.push(0x28);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    out.push(0);
}

fn push_int(out: &mut Vec<u8>, value: i32) {
    out.push(0x05);
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_vec3(out: &mut Vec<u8>, value: [f32; 3]) {
    out.push(0x20);
    for axis in value {
        out.extend_from_slice(&axis.to_le_bytes());
    }
}

fn header() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"BBST");
    out.extend_from_slice(&[0u8; 12]);
    out.extend_from_slice(&118u32.to_le_bytes());
    out.push(0x0b);
    out.push(0x01);
    assert_eq!(out.len(), STREAM_AT);
    out
}

/// 造一份最小 BST：头部 + 一条舞台头记录 + 一条碰撞板 + 一条地形块。
fn sample() -> Vec<u8> {
    let mut out = header();

    // 舞台头记录
    push_text(&mut out, "BT_STAGE00.fmd");
    push_text(&mut out, "BT_STAGE00.ftx");
    push_vec3(&mut out, [1900.0, 0.0, 1900.0]);
    push_vec3(&mut out, [-1600.0, 0.0, -1400.0]);
    push_vec3(&mut out, [1700.0, 0.0, 1700.0]);
    push_vec3(&mut out, [-1400.0, 0.0, -1200.0]);
    out.push(RECORD_END);

    // 碰撞板记录
    push_int(&mut out, 1);
    push_int(&mut out, 0);
    push_int(&mut out, 0);
    push_text(&mut out, "");
    out.push(0x0b);
    out.push(0);
    out.push(0x0b);
    out.push(0);
    push_int(&mut out, 30);
    push_vec3(&mut out, [483.0, 0.0, 48.0]);
    push_int(&mut out, 0);
    out.push(0x33);
    out.extend_from_slice(&4u32.to_le_bytes());
    for corner in [45, 162, 327, 593] {
        push_int(&mut out, corner);
    }
    out.push(RECORD_END);

    // 地形块记录：4 角 + 名字 + 若干填充
    for corner in [
        [0.0f32, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ] {
        push_vec3(&mut out, corner);
    }
    push_text(&mut out, "379");
    for _ in 0..16 {
        push_int(&mut out, 0);
    }
    out.push(RECORD_END);
    out
}

#[test]
fn parses_stage_header_and_records() {
    let bytes = sample();
    let bst = parse(&bytes);
    assert_eq!(bst.names, vec!["BT_STAGE00.fmd", "BT_STAGE00.ftx"]);
    assert_eq!(bst.records.len(), 3);
    assert_eq!(bst.end, bytes.len());
    assert!(bst.unknown_tag.is_none());
    assert_eq!(bst.plates.len(), 1);
    assert_eq!(bst.plates[0].id, 30);
    assert_eq!(bst.plates[0].corners, vec![45, 162, 327, 593]);
    assert_eq!(bst.quads.len(), 1);
    assert_eq!(bst.quads[0].name, "379");
}

#[test]
fn record_array_is_detected() {
    // 两条碰撞板放进记录数组：`33 02` + record + record + END
    let mut out = header();
    push_int(&mut out, 621);
    out.push(0x33);
    out.extend_from_slice(&2u32.to_le_bytes());
    for id in [30, 31] {
        push_int(&mut out, 1);
        push_int(&mut out, 0);
        push_int(&mut out, 0);
        out.push(0x28);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.push(0);
        out.push(0x0b);
        out.push(0);
        out.push(0x0b);
        out.push(0);
        push_int(&mut out, id);
        push_vec3(&mut out, [1.0, 0.0, 2.0]);
        push_int(&mut out, 0);
        out.push(0x33);
        out.extend_from_slice(&2u32.to_le_bytes());
        push_int(&mut out, 7);
        push_int(&mut out, 8);
        out.push(RECORD_END);
    }
    out.push(RECORD_END);

    let bst = parse(&out);
    assert!(bst.unknown_tag.is_none(), "应当整体闭合");
    assert_eq!(bst.end, out.len());
    assert_eq!(bst.plates.len(), 2);
    assert_eq!(bst.plates[1].id, 31);
    match &bst.records[0] {
        Node::Record(items) => match &items[1] {
            Node::Array { grouped, items, .. } => {
                assert!(*grouped, "应当识别成记录数组");
                assert_eq!(items.len(), 2);
            }
            other => panic!("第二项应当是数组：{other:?}"),
        },
        other => panic!("应当是记录：{other:?}"),
    }
}

#[test]
fn truncated_input_never_panics() {
    let bytes = sample();
    for len in 0..bytes.len() {
        let _ = parse(&bytes[..len]);
        let _ = build(&bytes[..len]);
    }
}
