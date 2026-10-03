use cancancan_core::{Ability, CanCanError, Condition, DbValue};
use cancancan_seaorm::{ColumnMap, ColumnType, accessible_by, condition_to_sea};
use sea_query::{
    Alias, Condition as SeaCondition, PostgresQueryBuilder, Query, SelectStatement, SimpleExpr,
    SqliteQueryBuilder,
};
use std::collections::HashMap;
use std::rc::Rc;

const USER_UUID: &str = "5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f";

fn columns() -> ColumnMap {
    HashMap::from([
        ("id".to_owned(), ColumnType::Uuid),
        ("user_id".to_owned(), ColumnType::Uuid),
        ("knowledge_base_id".to_owned(), ColumnType::Uuid),
        ("legacy_id".to_owned(), ColumnType::BigInt),
        ("published".to_owned(), ColumnType::Bool),
        ("title".to_owned(), ColumnType::Text),
    ])
}

fn eq(field: &str, value: DbValue) -> Condition {
    Condition::Eq {
        field: field.to_owned(),
        value,
    }
}

fn render_expr(expr: SimpleExpr) -> String {
    let mut statement = Query::select()
        .column(Alias::new("id"))
        .from(Alias::new("posts"))
        .to_owned();
    statement.and_where(expr);
    statement.to_string(PostgresQueryBuilder)
}

fn render_cond(condition: SeaCondition) -> String {
    let mut statement = Query::select()
        .column(Alias::new("id"))
        .from(Alias::new("posts"))
        .to_owned();
    statement.cond_where(condition);
    statement.to_string(PostgresQueryBuilder)
}

#[test]
fn uuid_primary_key_binds_natively() {
    let expr = condition_to_sea(&eq("id", DbValue::from(USER_UUID)), "posts", &columns()).unwrap();
    let sql = render_expr(expr);
    assert!(
        sql.contains("'5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f'"),
        "uuid must render quoted: {sql}"
    );
    assert!(sql.contains("\"posts\".\"id\""), "qualified column: {sql}");
}

#[test]
fn uuid_foreign_key_match_and_nullable_link() {
    let expr = condition_to_sea(
        &eq("user_id", DbValue::from(USER_UUID)),
        "posts",
        &columns(),
    )
    .unwrap();
    assert!(render_expr(expr).contains(USER_UUID));

    let nullable =
        condition_to_sea(&eq("knowledge_base_id", DbValue::Null), "posts", &columns()).unwrap();
    assert!(render_expr(nullable).contains("IS NULL"));
}

#[test]
fn integer_and_bigint_ids_bind() {
    let legacy = condition_to_sea(&eq("legacy_id", DbValue::Int(42)), "posts", &columns()).unwrap();
    assert!(render_expr(legacy).contains("= 42"));

    let huge = condition_to_sea(
        &eq("legacy_id", DbValue::Int(i64::MAX)),
        "posts",
        &columns(),
    )
    .unwrap();
    assert!(render_expr(huge).contains(&i64::MAX.to_string()));
}

#[test]
fn invalid_uuid_string_is_rejected() {
    let error =
        condition_to_sea(&eq("id", DbValue::from("not-a-uuid")), "posts", &columns()).unwrap_err();
    assert_eq!(error, CanCanError::AttributeArgument);
}

#[test]
fn string_rejected_on_bigint_column() {
    let error =
        condition_to_sea(&eq("legacy_id", DbValue::from("one")), "posts", &columns()).unwrap_err();
    assert_eq!(error, CanCanError::AttributeArgument);
}

#[test]
fn unknown_field_is_rejected() {
    let error = condition_to_sea(&eq("missing", DbValue::Int(1)), "posts", &columns()).unwrap_err();
    assert_eq!(error, CanCanError::AttributeArgument);
}

#[test]
fn in_list_of_uuids_and_empty_in() {
    let other = "f47ac10b-58cc-4372-a567-0e02b2c3d479";
    let expr = condition_to_sea(
        &Condition::In {
            field: "id".to_owned(),
            values: vec![DbValue::from(USER_UUID), DbValue::from(other)],
        },
        "posts",
        &columns(),
    )
    .unwrap();
    let sql = render_expr(expr);
    assert!(sql.contains("IN ("), "{sql}");
    assert!(sql.contains(USER_UUID), "{sql}");
    assert!(sql.contains(other), "{sql}");

    let empty = condition_to_sea(
        &Condition::In {
            field: "id".to_owned(),
            values: vec![],
        },
        "posts",
        &columns(),
    )
    .unwrap();
    assert!(render_expr(empty).contains("1 = 0"));
}

#[test]
fn combinators_and_not_raw_sql() {
    let expr = condition_to_sea(
        &Condition::And(vec![
            eq("published", DbValue::Bool(true)),
            Condition::Range {
                field: "legacy_id".to_owned(),
                min: DbValue::Int(1),
                max: DbValue::Int(10),
            },
        ]),
        "posts",
        &columns(),
    )
    .unwrap();
    let sql = render_expr(expr);
    assert!(sql.contains("AND"), "{sql}");
    assert!(sql.contains("BETWEEN 1 AND 10"), "{sql}");

    let negated = render_expr(
        condition_to_sea(
            &Condition::Not(Box::new(eq("published", DbValue::Bool(false)))),
            "posts",
            &columns(),
        )
        .unwrap(),
    );
    assert!(negated.contains("NOT"), "negated: {negated}");

    let raw = condition_to_sea(
        &Condition::RawSql("published = TRUE".to_owned()),
        "posts",
        &columns(),
    )
    .unwrap();
    assert!(render_expr(raw).contains("published = TRUE"));
}

#[test]
fn accessible_by_composes_allow_or_deny_not() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("user_id", DbValue::from(USER_UUID)),
        )
        .unwrap();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("published", DbValue::Bool(true)),
        )
        .unwrap();
    ability
        .cannot_where(Some("read"), Some("Post"), eq("legacy_id", DbValue::Int(9)))
        .unwrap();

    let condition = accessible_by(&ability, "read", "Post", "posts", &columns()).unwrap();
    let sql = render_cond(condition);
    assert!(sql.contains("OR"), "{sql}");
    assert!(sql.contains("AND"), "{sql}");
    assert!(sql.contains("NOT"), "{sql}");
    assert!(sql.contains(USER_UUID), "{sql}");
}

#[test]
fn accessible_by_no_rules_matches_nothing() {
    let condition = accessible_by(&Ability::new(), "read", "Post", "posts", &columns()).unwrap();
    let sql = render_cond(condition);
    assert!(sql.contains("1 = 0"), "{sql}");
}

#[test]
fn accessible_by_catch_all_matches_everything() {
    let mut ability = Ability::new();
    ability.can(Some("manage"), Some("all")).unwrap();
    let condition = accessible_by(&ability, "read", "Post", "posts", &columns()).unwrap();
    let sql = render_cond(condition);
    assert!(sql.contains("1 = 1"), "{sql}");
}

#[test]
fn nested_and_matcher_rules_are_rejected() {
    let mut nested = Ability::new();
    nested
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Nested {
                relation: "author".to_owned(),
                condition: Box::new(eq("title", DbValue::from("t"))),
            },
        )
        .unwrap();
    assert_eq!(
        accessible_by(&nested, "read", "Post", "posts", &columns()).unwrap_err(),
        CanCanError::WrongAssociation("author".to_owned())
    );

    let mut matcher = Ability::new();
    matcher
        .can_matching(Some("read"), Some("Post"), Rc::new(|_| true))
        .unwrap();
    assert_eq!(
        accessible_by(&matcher, "read", "Post", "posts", &columns()).unwrap_err(),
        CanCanError::BlockInQuery
    );
}

#[test]
fn sqlite_builder_renders_same_condition() {
    let expr = condition_to_sea(&eq("legacy_id", DbValue::Int(7)), "posts", &columns()).unwrap();
    let mut statement: SelectStatement = Query::select()
        .column(Alias::new("id"))
        .from(Alias::new("posts"))
        .to_owned();
    statement.and_where(expr);
    let sql = statement.to_string(SqliteQueryBuilder);
    assert!(sql.contains("= 7"), "{sql}");
}
