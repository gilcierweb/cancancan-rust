use cancancan_core::{Ability, Condition, DbValue};
use cancancan_diesel::accessible_by_sql;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Bool;

diesel::table! {
    posts (id) {
        id -> Integer,
        user_id -> Integer,
        published -> Bool,
        title -> Text,
    }
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
    let fragment = accessible_by_sql(ability, "read", "Post", "posts").unwrap();
    let mut connection = seed();
    posts::table
        .select(posts::id)
        .filter(sql::<Bool>(&fragment))
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
                Condition::Range {
                    field: "user_id".to_owned(),
                    min: DbValue::Int(1),
                    max: DbValue::Int(1),
                },
            ]),
        )
        .unwrap();
    assert_eq!(visible_ids(&ability), vec![1, 3]);
}

#[test]
fn no_rules_match_nothing() {
    assert_eq!(visible_ids(&Ability::new()), Vec::<i32>::new());
}

#[test]
fn manage_all_matches_everything() {
    let mut ability = Ability::new();
    ability.can(Some("manage"), Some("all")).unwrap();
    assert_eq!(visible_ids(&ability), vec![1, 2, 3, 4]);
}
