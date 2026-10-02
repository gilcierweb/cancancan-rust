use cancancan_core::{Condition, DbValue, Rule, compress};

fn eq_user(user_id: i64) -> Condition {
    Condition::Eq {
        field: "user_id".to_owned(),
        value: DbValue::Int(user_id),
    }
}

#[test]
fn duplicate_conditions_are_removed_keeping_the_last() {
    let rules = vec![
        Rule::allow_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
        Rule::allow_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
    ];
    assert_eq!(compress(rules).len(), 1);
}

#[test]
fn rules_after_last_catch_all_with_same_behavior_are_dropped() {
    let rules = vec![
        Rule::allow(Some("read".to_owned()), Some("Post".to_owned())).unwrap(),
        Rule::allow_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
        Rule::allow(None::<String>, None::<String>).unwrap(),
        Rule::allow_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(2)).unwrap(),
    ];
    let compressed = compress(rules);
    assert_eq!(compressed.len(), 2);
    assert!(compressed[1].is_catch_all());
}

#[test]
fn trailing_deny_catch_all_is_removed() {
    let rules = vec![
        Rule::allow(None::<String>, None::<String>).unwrap(),
        Rule::deny(None::<String>, None::<String>).unwrap(),
    ];
    let compressed = compress(rules);
    assert_eq!(compressed.len(), 1);
    assert!(compressed[0].allows());
}

#[test]
fn distinct_rules_survive_compression() {
    let rules = vec![
        Rule::allow_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
        Rule::allow_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(2)).unwrap(),
    ];
    assert_eq!(compress(rules).len(), 2);
}
