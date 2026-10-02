mod common;

use cancancan_core::{Actions, CanCanError, Condition, DbValue, Rule};
use common::Post;
use std::rc::Rc;

#[test]
fn can_rule_reports_its_parts() {
    let rule = Rule::can_where(
        Some("read".to_owned()),
        Some("Post".to_owned()),
        Condition::Eq {
            field: "user_id".to_owned(),
            value: DbValue::Int(1),
        },
    )
    .unwrap();

    assert!(rule.allows());
    assert_eq!(rule.actions(), &["read".to_owned()]);
    assert_eq!(rule.subjects(), &["Post".to_owned()]);
    assert!(!rule.has_matcher());
    assert!(!rule.is_catch_all());
}

#[test]
fn rule_without_action_matches_everything() {
    let rule = Rule::can(None::<String>, None::<String>).unwrap();
    let actions = Actions::new();
    assert!(rule.covers_action(&actions, "destroy"));
    assert!(rule.matches_subject("Anything"));
    assert!(rule.is_catch_all());
}

#[test]
fn manage_action_and_all_subject_are_wildcards() {
    let actions = Actions::new();
    let rule = Rule::can(Some("manage".to_owned()), Some("Post".to_owned())).unwrap();
    assert!(rule.covers_action(&actions, "destroy"));
    assert!(!rule.matches_subject("Comment"));

    let rule = Rule::can(Some("read".to_owned()), Some("all".to_owned())).unwrap();
    assert!(rule.matches_subject("Post"));
}

#[test]
fn rule_defined_on_alias_covers_mapped_actions() {
    let actions = Actions::new();
    let rule = Rule::can(Some("read".to_owned()), Some("Post".to_owned())).unwrap();
    assert!(rule.covers_action(&actions, "index"));
    assert!(rule.covers_action(&actions, "show"));
    assert!(!rule.covers_action(&actions, "destroy"));
    assert!(rule.is_relevant(&actions, "show", "Post"));
    assert!(!rule.is_relevant(&actions, "show", "Comment"));
}

#[test]
fn cannot_catch_all_is_detected() {
    let rule = Rule::cannot(None::<String>, None::<String>).unwrap();
    assert!(rule.is_deny_catch_all());

    let rule = Rule::can(None::<String>, None::<String>).unwrap();
    assert!(!rule.is_deny_catch_all());
}

#[test]
fn block_matcher_rules_match_through_matcher() {
    let rule = Rule::can_matching(
        Some("read".to_owned()),
        Some("Post".to_owned()),
        Rc::new(|_| true),
    )
    .unwrap();
    assert!(rule.has_matcher());
    assert!(!rule.is_catch_all());
    assert!(rule.matches_instance(&Post::owned(1, 1)));
}

#[test]
fn raw_sql_rule_is_detected() {
    let rule = Rule::can_where(
        Some("read".to_owned()),
        Some("Post".to_owned()),
        Condition::RawSql("published".to_owned()),
    )
    .unwrap();
    assert!(rule.has_raw_sql());
    assert!(!rule.is_catch_all());
}

#[test]
fn action_without_subject_is_rejected_for_every_constructor() {
    assert_eq!(
        Rule::can(Some("read".to_owned()), None::<String>).unwrap_err(),
        CanCanError::ActionWithoutSubject
    );
    assert_eq!(
        Rule::cannot_where(Some("read".to_owned()), None::<String>, Condition::All,).unwrap_err(),
        CanCanError::ActionWithoutSubject
    );
}

#[test]
fn attribute_matching_falls_back_to_behavior() {
    let plain = Rule::can(Some("read".to_owned()), Some("Post".to_owned())).unwrap();
    assert!(plain.matches_attribute(Some("title")));
    assert!(plain.matches_attribute(None));

    let listed = Rule::can(Some("read".to_owned()), Some("Post".to_owned()))
        .unwrap()
        .with_attributes(vec!["title".to_owned()]);
    assert!(listed.matches_attribute(Some("title")));
    assert!(!listed.matches_attribute(Some("user_id")));
    assert!(listed.matches_attribute(None));

    let denied = Rule::cannot(Some("read".to_owned()), Some("Post".to_owned()))
        .unwrap()
        .with_attributes(vec!["title".to_owned()]);
    assert!(!denied.matches_attribute(None));
}
