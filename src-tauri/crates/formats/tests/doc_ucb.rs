use formats::document::ucb::*;

#[test]
fn rejects_short_file() {
    for len in 0..0x40 {
        let arena = build(&vec![0u8; len]);
        assert!(arena.total_nodes() > 0);
    }
}

#[test]
fn checks_alignment_and_grouping() {
    let bytes = vec![0u8; RECORD * GROUP_SIZE * 2];
    let arena = build(&bytes);
    let checks: Vec<_> = arena
        .checks()
        .iter()
        .map(|c| (c.id.as_str(), c.ok))
        .collect();
    assert!(checks.contains(&("ucb.align", true)));
    assert!(checks.contains(&("ucb.groups", true)));
    assert!(checks.contains(&("ucb.group", false)));
}
