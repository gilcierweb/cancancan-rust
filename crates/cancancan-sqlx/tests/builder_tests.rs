use cancancan_core::{Ability, CanCanError, Condition, DbValue};
use cancancan_sqlx::{ColumnMap, ColumnType, accessible_by, push_condition};
use sqlx::QueryBuilder;
use std::collections::HashMap;
use std::sync::Arc;

const USER_UUID: &str = "5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f";

fn columns() -> ColumnMap {
    HashMap::from([
        ("id".to_owned(), ColumnType::BigInt),
        ("user_id".to_owned(), ColumnType::Uuid),
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

fn postgres_sql(condition: &Condition) -> String {
    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    crate::push_condition(&mut builder, condition, "posts", &columns()).unwrap();
    builder.sql().to_owned()
}

#[test]
fn postgres_placeholders_use_dollar_numbers() {
    let sql = postgres_sql(&eq("id", DbValue::Int(1)));
    assert_eq!(sql, r#""posts"."id" = $1"#);

    let sql = postgres_sql(&Condition::And(vec![
        eq("id", DbValue::Int(1)),
        eq("title", DbValue::from("hello")),
    ]));
    assert_eq!(
        sql, r#"("posts"."id" = $1) AND ("posts"."title" = $2)"#,
        "and: {sql}"
    );
}

#[test]
fn mysql_placeholders_use_question_marks() {
    let mut builder = QueryBuilder::<sqlx::MySql>::new("");
    push_condition(
        &mut builder,
        &eq("id", DbValue::Int(1)),
        "posts",
        &columns(),
    )
    .unwrap();
    assert_eq!(builder.sql(), r#""posts"."id" = ?"#);
}

#[test]
fn uuid_value_validated_as_text_bind() {
    let sql = postgres_sql(&eq("user_id", DbValue::from(USER_UUID)));
    assert_eq!(sql, r#""posts"."user_id" = $1"#);

    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    let error = push_condition(
        &mut builder,
        &eq("user_id", DbValue::from("not-a-uuid")),
        "posts",
        &columns(),
    )
    .unwrap_err();
    assert_eq!(error, CanCanError::AttributeArgument);
}

#[test]
fn null_range_in_not_and_raw_sql_shape() {
    assert_eq!(
        postgres_sql(&eq("title", DbValue::Null)),
        r#""posts"."title" IS NULL"#
    );
    assert_eq!(
        postgres_sql(&Condition::Ne {
            field: "title".to_owned(),
            value: DbValue::Null,
        }),
        r#""posts"."title" IS NOT NULL"#
    );
    assert_eq!(
        postgres_sql(&Condition::Range {
            field: "id".to_owned(),
            min: DbValue::Int(1),
            max: DbValue::Int(10),
        }),
        r#""posts"."id" BETWEEN $1 AND $2"#
    );
    assert_eq!(
        postgres_sql(&Condition::In {
            field: "id".to_owned(),
            values: vec![DbValue::Int(1), DbValue::Int(2)],
        }),
        r#""posts"."id" IN ($1, $2)"#
    );
    assert_eq!(
        postgres_sql(&Condition::Not(Box::new(eq(
            "published",
            DbValue::Bool(true)
        )))),
        r#"NOT ("posts"."published" = $1)"#
    );
    assert_eq!(
        postgres_sql(&Condition::RawSql("published".to_owned())),
        "(published)"
    );
}

#[test]
fn empty_in_never_matches_and_negated_empty_always_matches() {
    assert_eq!(
        postgres_sql(&Condition::In {
            field: "id".to_owned(),
            values: vec![],
        }),
        "1 = 0"
    );
    assert_eq!(
        postgres_sql(&Condition::Ne {
            field: "id".to_owned(),
            value: DbValue::List(vec![]),
        }),
        "1 = 1"
    );
}

#[test]
fn unknown_field_and_identifier_injection_rejected() {
    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    assert_eq!(
        push_condition(
            &mut builder,
            &eq("nope", DbValue::Int(1)),
            "posts",
            &columns()
        )
        .unwrap_err(),
        CanCanError::AttributeArgument
    );
    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    let malicious = eq("1) OR 1=1--", DbValue::Int(1));
    assert_eq!(
        push_condition(&mut builder, &malicious, "posts", &columns()).unwrap_err(),
        CanCanError::AttributeArgument
    );
}

#[test]
fn nested_and_matcher_rejected() {
    assert_eq!(
        postgres_sql_err(&Condition::Nested {
            relation: "author".to_owned(),
            condition: Box::new(eq("id", DbValue::Int(1))),
        }),
        CanCanError::WrongAssociation("author".to_owned())
    );

    let mut ability = Ability::new();
    ability
        .can_matching(Some("read"), Some("Post"), Arc::new(|_| true))
        .unwrap();
    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    assert_eq!(
        accessible_by(&mut builder, &ability, "read", "Post", "posts", &columns()).unwrap_err(),
        CanCanError::BlockInQuery
    );
}

fn postgres_sql_err(condition: &Condition) -> CanCanError {
    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    crate::push_condition(&mut builder, condition, "posts", &columns()).unwrap_err()
}

#[test]
fn accessible_by_composes_allow_or_and_deny_not() {
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
        .cannot_where(Some("read"), Some("Post"), eq("id", DbValue::Int(9)))
        .unwrap();

    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    accessible_by(&mut builder, &ability, "read", "Post", "posts", &columns()).unwrap();
    let sql = builder.sql().to_owned();
    assert_eq!(
        sql,
        r#"(("posts"."published" = $1) OR ("posts"."user_id" = $2)) AND NOT (("posts"."id" = $3))"#
    );
}

#[test]
fn accessible_by_no_rules_matches_nothing() {
    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    accessible_by(
        &mut builder,
        &Ability::new(),
        "read",
        "Post",
        "posts",
        &columns(),
    )
    .unwrap();
    assert_eq!(builder.sql(), "1 = 0");
}

#[test]
fn accessible_by_single_allow_has_no_extra_parens() {
    let mut ability = Ability::new();
    ability
        .can_where(Some("read"), Some("Post"), eq("id", DbValue::Int(1)))
        .unwrap();
    let mut builder = QueryBuilder::<sqlx::Postgres>::new("");
    accessible_by(&mut builder, &ability, "read", "Post", "posts", &columns()).unwrap();
    assert_eq!(builder.sql(), r#""posts"."id" = $1"#);
}
