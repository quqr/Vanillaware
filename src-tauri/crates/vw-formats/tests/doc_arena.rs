use vw_formats::doc::arena::*;

#[test]
fn pages_children_without_inlining() {
    let mut builder = Builder::new();
    let root = builder.root("root");
    let group = builder.group(root, Some("items"), "items");
    for index in 0..5 {
        builder.field(group, &index.to_string(), format!("v{index}"));
    }
    let arena = builder.finish();

    let page = arena.page(1, group, 1, 2).unwrap();
    assert_eq!(page.total, 5);
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].value.as_deref(), Some("v1"));
    assert_eq!(page.items[0].child_count, 0);
    assert_eq!(arena.total_nodes(), 7);
}

#[test]
fn invalid_node_returns_none() {
    let arena = Builder::new().finish();
    assert!(arena.page(1, 99, 0, 100).is_none());
}
