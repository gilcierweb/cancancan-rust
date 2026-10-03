use cancancan_core::{Condition, DbValue, Rule, compress};

fn eq_user(user_id: i64) -> Condition {
    Condition::Eq {
        field: "user_id".to_owned(),
        value: DbValue::Int(user_id),
    }
}

// compress() receives relevant rules latest-defined-first (like the gem's
// `RulesCompressor.new(rules.reverse)`) and returns them in the same order.

#[test]
fn duplicate_conditions_are_removed_keeping_the_last() {
    let rules = vec![
        Rule::can_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
        Rule::can_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
    ];
    assert_eq!(compress(rules).len(), 1);
}

#[test]
fn catch_all_subsumes_earlier_rules_and_later_same_behavior() {
    // definition order: can(eq2), can(all), can(eq1), can
    // the catch-all makes everything else redundant
    let rules = vec![
        Rule::can_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(2)).unwrap(),
        Rule::can(Some("read".to_owned()), Some("Post".to_owned())).unwrap(),
        Rule::can_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
        Rule::can(None::<String>, None::<String>).unwrap(),
    ];
    let latest_first: Vec<Rule> = rules.into_iter().rev().collect();
    let compressed = compress(latest_first);
    assert_eq!(compressed.len(), 1);
    assert!(compressed[0].is_catch_all());
}

#[test]
fn later_deny_catch_all_wins_over_everything() {
    // definition order: can(all), cannot(all) -> nothing readable
    let rules = vec![
        Rule::can(None::<String>, None::<String>).unwrap(),
        Rule::cannot(None::<String>, None::<String>).unwrap(),
    ];
    let latest_first: Vec<Rule> = rules.into_iter().rev().collect();
    assert!(compress(latest_first).is_empty());
}

#[test]
fn leading_deny_catch_all_is_removed() {
    // definition order: cannot(all), can(eq1) -> [can(eq1)]
    let rules = vec![
        Rule::cannot(None::<String>, None::<String>).unwrap(),
        Rule::can_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
    ];
    let latest_first: Vec<Rule> = rules.into_iter().rev().collect();
    let compressed = compress(latest_first);
    assert_eq!(compressed.len(), 1);
    assert!(compressed[0].allows());
}

#[test]
fn later_opposite_behavior_survives_catch_all() {
    // definition order: cannot(eq1), can(all), cannot(eq2)
    // gem => [can(all), cannot(eq2)]
    let rules = vec![
        Rule::cannot_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
        Rule::can(Some("read".to_owned()), Some("Post".to_owned())).unwrap(),
        Rule::cannot_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(2)).unwrap(),
    ];
    let latest_first: Vec<Rule> = rules.into_iter().rev().collect();
    let compressed = compress(latest_first);
    // latest-first output: [cannot(eq2), can(all)]
    assert_eq!(compressed.len(), 2);
    assert!(!compressed[0].allows());
    assert!(compressed[1].allows() && compressed[1].is_catch_all());
}

#[test]
fn distinct_rules_survive_compression() {
    let rules = vec![
        Rule::can_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(1)).unwrap(),
        Rule::can_where(Some("read".to_owned()), Some("Post".to_owned()), eq_user(2)).unwrap(),
    ];
    assert_eq!(compress(rules).len(), 2);
}
