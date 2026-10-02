use cancancan_core::{Ability, CanCanError, Condition, DbValue};
use cancancan_diesel::{accessible_by_sql, condition_sql};
use rstest::rstest;

fn eq(field: &str, value: DbValue) -> Condition {
    Condition::Eq {
        field: field.to_owned(),
        value,
    }
}

#[rstest]
#[case::eq_int(eq("user_id", DbValue::Int(1)), r#""posts"."user_id" = 1"#)]
#[case::eq_text_escapes_quotes(
    eq("title", DbValue::from("o'brien")),
    r#""posts"."title" = 'o''brien'"#
)]
#[case::eq_bool(eq("published", DbValue::Bool(true)), r#""posts"."published" = TRUE"#)]
#[case::eq_null(eq("title", DbValue::Null), r#""posts"."title" IS NULL"#)]
#[case::ne_null(
    Condition::Ne { field: "title".to_owned(), value: DbValue::Null },
    r#""posts"."title" IS NOT NULL"#,
)]
#[case::in_list(
    Condition::In { field: "id".to_owned(), values: vec![DbValue::Int(1), DbValue::Int(2)] },
    r#""posts"."id" IN (1, 2)"#,
)]
#[case::in_empty_matches_nothing(
    Condition::In { field: "id".to_owned(), values: vec![] },
    "1 = 0",
)]
#[case::range(
    Condition::Range { field: "id".to_owned(), min: DbValue::Int(1), max: DbValue::Int(10) },
    r#""posts"."id" BETWEEN 1 AND 10"#,
)]
#[case::nested_uses_relation_name(
    Condition::Nested {
        relation: "author".to_owned(),
        condition: Box::new(eq("name", DbValue::from("ada"))),
    },
    r#""author"."name" = 'ada'"#,
)]
#[case::not(
    Condition::Not(Box::new(eq("published", DbValue::Bool(true)))),
    r#"NOT ("posts"."published" = TRUE)"#
)]
#[case::and(
    Condition::And(vec![
        eq("user_id", DbValue::Int(1)),
        eq("published", DbValue::Bool(true)),
    ]),
    r#"("posts"."user_id" = 1) AND ("posts"."published" = TRUE)"#,
)]
fn renders_conditions(#[case] condition: Condition, #[case] expected: &str) {
    assert_eq!(condition_sql(&condition, "posts").unwrap(), expected);
}

#[test]
fn rejects_invalid_identifiers() {
    assert_eq!(
        condition_sql(&eq("user Ergänzung", DbValue::Int(1)), "posts").unwrap_err(),
        CanCanError::AttributeArgument
    );
    assert_eq!(
        condition_sql(&eq("user_id", DbValue::Int(1)), "posts; DROP TABLE posts").unwrap_err(),
        CanCanError::AttributeArgument
    );
    assert_eq!(
        condition_sql(
            &Condition::Nested {
                relation: "a-b".to_owned(),
                condition: Box::new(eq("id", DbValue::Int(1))),
            },
            "posts",
        )
        .unwrap_err(),
        CanCanError::WrongAssociation("a-b".to_owned())
    );
}

#[test]
fn accessible_by_combines_allow_or_and_deny_not() {
    let mut ability = Ability::new();
    ability
        .can_where(Some("read"), Some("Post"), eq("user_id", DbValue::Int(1)))
        .unwrap();
    ability
        .can_where(Some("read"), Some("Post"), eq("published", DbValue::Bool(true)))
        .unwrap();
    ability
        .cannot_where(Some("read"), Some("Post"), eq("id", DbValue::Int(9)))
        .unwrap();

    assert_eq!(
        accessible_by_sql(&ability, "read", "Post", "posts").unwrap(),
        r#"(("posts"."published" = TRUE) OR ("posts"."user_id" = 1)) AND NOT (("posts"."id" = 9))"#,
    );
}

#[test]
fn accessible_by_without_allow_matches_nothing() {
    let ability = Ability::new();
    assert_eq!(
        accessible_by_sql(&ability, "read", "Post", "posts").unwrap(),
        "1 = 0"
    );
}

#[test]
fn accessible_by_with_catch_all_matches_everything() {
    let mut ability = Ability::new();
    ability.can(Some("manage"), Some("all")).unwrap();
    assert_eq!(
        accessible_by_sql(&ability, "read", "Post", "posts").unwrap(),
        "1 = 1"
    );
}

#[test]
fn accessible_by_rejects_matcher_rules() {
    use std::rc::Rc;
    let mut ability = Ability::new();
    ability
        .can_matching(Some("read"), Some("Post"), Rc::new(|_| true))
        .unwrap();
    assert_eq!(
        accessible_by_sql(&ability, "read", "Post", "posts").unwrap_err(),
        CanCanError::BlockInQuery
    );
}
