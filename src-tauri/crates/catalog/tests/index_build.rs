use catalog::format;
use catalog::index::*;

fn entry(name: &str) -> IndexEntry {
    let entry_format =
        format::lookup(name.rsplit('.').next().unwrap_or_default()).unwrap_or(&format::FORMATS[0]);
    IndexEntry::new(format!("C:\\x\\{name}"), name.to_string(), entry_format)
}

#[test]
fn search_is_case_insensitive_and_reports_truncation() {
    let entries = vec![entry("Alpha.asb"), entry("alpha2.asb"), entry("Beta.ftx")];
    let hit = search(&entries, "ALPHA", 10);
    assert_eq!(hit.total, 2);
    assert!(!hit.truncated);

    let truncated = search(&entries, "alpha", 1);
    assert_eq!(truncated.items.len(), 1);
    assert_eq!(truncated.total, 2);
    assert!(truncated.truncated);
}

#[test]
fn empty_query_matches_nothing() {
    let entries = vec![entry("Alpha.asb")];
    assert_eq!(search(&entries, "   ", 10).total, 0);
}
