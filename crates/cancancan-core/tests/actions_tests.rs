use cancancan_core::Actions;

#[test]
fn defaults_expand_read_create_and_update() {
    let actions = Actions::new();
    assert_eq!(actions.expand("read"), vec!["read", "index", "show"]);
    assert_eq!(actions.expand("create"), vec!["create", "new"]);
    assert_eq!(actions.expand("update"), vec!["update", "edit"]);
    assert_eq!(actions.expand("destroy"), vec!["destroy"]);
}

#[test]
fn custom_alias_expands_transitively() {
    let mut actions = Actions::new();
    actions.alias_action(["read"], "browse");
    actions.alias_action(["browse"], "peek");

    let expanded = actions.expand("peek");
    assert!(expanded.contains(&"peek".to_owned()));
    assert!(expanded.contains(&"browse".to_owned()));
    assert!(expanded.contains(&"read".to_owned()));
    assert!(expanded.contains(&"index".to_owned()));
    assert!(expanded.contains(&"show".to_owned()));
}

#[test]
fn aliases_for_lists_reverse_lookup() {
    let actions = Actions::new();
    assert_eq!(actions.aliases_for("show"), vec!["read"]);
    assert!(actions.aliases_for("destroy").is_empty());
}

#[test]
fn clear_removes_defaults() {
    let mut actions = Actions::new();
    actions.clear();
    assert_eq!(actions.expand("read"), vec!["read"]);
}
