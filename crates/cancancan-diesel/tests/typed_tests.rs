#![cfg(feature = "sqlite")]

use cancancan_core::{Ability, CanCanError, Condition, DbValue};
use cancancan_diesel::sqlite::accessible_by;
use cancancan_diesel::{ColumnMap, ColumnType};
use diesel::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

diesel::table! {
    posts (id) {
        id -> Integer,
        user_id -> Integer,
        published -> Bool,
        title -> Text,
    }
}

fn columns() -> ColumnMap {
    HashMap::from([
        ("id".to_owned(), ColumnType::Integer),
        ("user_id".to_owned(), ColumnType::Integer),
        ("published".to_owned(), ColumnType::Bool),
        ("title".to_owned(), ColumnType::Text),
    ])
}

fn seed() -> SqliteConnection {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();
    diesel::sql_query(
        "CREATE TABLE posts (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            published BOOLEAN NOT NULL,
            title TEXT NOT NULL
        )",
    )
    .execute(&mut connection)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO posts (id, user_id, published, title) VALUES
            (1, 1, TRUE, 'hello'),
            (2, 2, FALSE, 'o''brien'),
            (3, 1, FALSE, 'draft'),
            (4, 2, TRUE, 'news')",
    )
    .execute(&mut connection)
    .unwrap();
    connection
}

fn visible_ids(ability: &Ability) -> Vec<i32> {
    let predicate =
        accessible_by::<posts::table>(ability, "read", "Post", "posts", &columns()).unwrap();
    let mut connection = seed();
    posts::table
        .select(posts::id)
        .filter(predicate)
        .order(posts::id)
        .load::<i32>(&mut connection)
        .unwrap()
}

fn eq(field: &str, value: DbValue) -> Condition {
    Condition::Eq {
        field: field.to_owned(),
        value,
    }
}

#[test]
fn owner_reads_own_posts() {
    let mut ability = Ability::new();
    ability
        .can_where(Some("read"), Some("Post"), eq("user_id", DbValue::Int(1)))
        .unwrap();
    assert_eq!(visible_ids(&ability), vec![1, 3]);
}

#[test]
fn deny_removes_unpublished_posts() {
    let mut ability = Ability::new();
    ability.can(Some("read"), Some("Post")).unwrap();
    ability
        .cannot_where(
            Some("read"),
            Some("Post"),
            eq("published", DbValue::Bool(false)),
        )
        .unwrap();
    assert_eq!(visible_ids(&ability), vec![1, 4]);
}

#[test]
fn quoted_title_matches() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("title", DbValue::from("o'brien")),
        )
        .unwrap();
    assert_eq!(visible_ids(&ability), vec![2]);
}

#[test]
fn combined_conditions_filter() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::And(vec![
                Condition::In {
                    field: "id".to_owned(),
                    values: vec![DbValue::Int(1), DbValue::Int(2), DbValue::Int(3)],
                },
                Condition::Not(Box::new(eq("published", DbValue::Bool(false)))),
            ]),
        )
        .unwrap();
    assert_eq!(visible_ids(&ability), vec![1]);
}

#[test]
fn no_rules_match_nothing() {
    assert_eq!(visible_ids(&Ability::new()), Vec::<i32>::new());
}

#[test]
fn unknown_field_is_rejected() {
    let mut ability = Ability::new();
    ability
        .can_where(Some("read"), Some("Post"), eq("nope", DbValue::Int(1)))
        .unwrap();
    assert_eq!(
        accessible_by::<posts::table>(&ability, "read", "Post", "posts", &columns()).err(),
        Some(CanCanError::AttributeArgument)
    );
}

#[test]
fn values_travel_as_bind_parameters() {
    let mut ability = Ability::new();
    ability
        .can_where(Some("read"), Some("Post"), eq("user_id", DbValue::Int(1)))
        .unwrap();
    let predicate =
        accessible_by::<posts::table>(&ability, "read", "Post", "posts", &columns()).unwrap();
    let query = posts::table.select(posts::id).filter(predicate);
    let debug = diesel::debug_query::<diesel::sqlite::Sqlite, _>(&query).to_string();
    assert!(
        debug.contains("`posts`.`user_id` = ?"),
        "expected placeholder: {debug}"
    );
    assert!(debug.contains("binds: [1]"), "expected bind value: {debug}");
}

#[cfg(feature = "postgres")]
mod uuid_postgres {
    use super::*;
    use cancancan_diesel::postgres::accessible_by as accessible_by_pg;

    const USER_UUID: &str = "5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f";

    fn uuid_columns() -> ColumnMap {
        HashMap::from([
            ("id".to_owned(), ColumnType::Uuid),
            ("user_id".to_owned(), ColumnType::Uuid),
            ("legacy_id".to_owned(), ColumnType::BigInt),
        ])
    }

    #[test]
    fn uuid_primary_key_binds_natively() {
        let mut ability = Ability::new();
        ability
            .can_where(
                Some("read"),
                Some("Post"),
                eq("id", DbValue::from(USER_UUID)),
            )
            .unwrap();
        let predicate =
            accessible_by_pg::<posts::table>(&ability, "read", "Post", "posts", &uuid_columns())
                .unwrap();
        let query = posts::table.select(posts::id).filter(predicate);
        let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
        assert!(debug.contains("= $1"), "expected bind placeholder: {debug}");
    }

    #[test]
    fn invalid_uuid_string_is_rejected() {
        let mut ability = Ability::new();
        ability
            .can_where(
                Some("read"),
                Some("Post"),
                eq("id", DbValue::from("not-a-uuid")),
            )
            .unwrap();
        assert_eq!(
            accessible_by_pg::<posts::table>(&ability, "read", "Post", "posts", &uuid_columns())
                .err(),
            Some(CanCanError::AttributeArgument)
        );
    }
}

#[test]
fn nested_condition_is_rejected() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Nested {
                relation: "author".to_owned(),
                condition: Box::new(eq("name", DbValue::from("ada"))),
            },
        )
        .unwrap();
    assert_eq!(
        accessible_by::<posts::table>(&ability, "read", "Post", "posts", &columns()).err(),
        Some(CanCanError::WrongAssociation("author".to_owned()))
    );
}

#[test]
fn matcher_rule_is_rejected() {
    let mut ability = Ability::new();
    ability
        .can_matching(Some("read"), Some("Post"), Arc::new(|_| true))
        .unwrap();
    assert_eq!(
        accessible_by::<posts::table>(&ability, "read", "Post", "posts", &columns()).err(),
        Some(CanCanError::BlockInQuery)
    );
}
