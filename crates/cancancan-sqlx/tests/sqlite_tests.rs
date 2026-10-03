use cancancan_core::{Ability, Condition, DbValue};
use cancancan_sqlx::{ColumnMap, ColumnType, accessible_by};
use sqlx::QueryBuilder;
use std::collections::HashMap;

const USER_UUID: &str = "5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f";
const OTHER_UUID: &str = "f47ac10b-58cc-4372-a567-0e02b2c3d479";

fn columns() -> ColumnMap {
    HashMap::from([
        ("id".to_owned(), ColumnType::BigInt),
        ("user_id".to_owned(), ColumnType::Uuid),
        ("published".to_owned(), ColumnType::Bool),
        ("title".to_owned(), ColumnType::Text),
    ])
}

async fn seed() -> sqlx::SqliteConnection {
    use sqlx::Connection as _;
    let mut connection = sqlx::SqliteConnection::connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE posts (
            id INTEGER PRIMARY KEY,
            user_id TEXT NOT NULL,
            published BOOLEAN NOT NULL,
            title TEXT NOT NULL
        )",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO posts (id, user_id, published, title) VALUES
            (1, '5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f', 1, 'hello'),
            (2, 'f47ac10b-58cc-4372-a567-0e02b2c3d479', 0, 'o''brien'),
            (3, '5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f', 0, 'draft'),
            (4, 'f47ac10b-58cc-4372-a567-0e02b2c3d479', 1, 'news')",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    connection
}

async fn visible_ids(ability: &Ability) -> Vec<i64> {
    let mut builder = QueryBuilder::<sqlx::Sqlite>::new("SELECT id FROM posts WHERE ");
    accessible_by(&mut builder, ability, "read", "Post", "posts", &columns()).unwrap();
    builder.push(" ORDER BY id");
    let mut connection = seed().await;
    builder
        .build_query_scalar::<i64>()
        .fetch_all(&mut connection)
        .await
        .unwrap()
}

fn eq(field: &str, value: DbValue) -> Condition {
    Condition::Eq {
        field: field.to_owned(),
        value,
    }
}

#[tokio::test]
async fn owner_reads_own_posts_via_uuid() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("user_id", DbValue::from(USER_UUID)),
        )
        .unwrap();
    assert_eq!(visible_ids(&ability).await, vec![1, 3]);
}

#[tokio::test]
async fn deny_removes_unpublished_posts() {
    let mut ability = Ability::new();
    ability.can(Some("read"), Some("Post")).unwrap();
    ability
        .cannot_where(
            Some("read"),
            Some("Post"),
            eq("published", DbValue::Bool(false)),
        )
        .unwrap();
    assert_eq!(visible_ids(&ability).await, vec![1, 4]);
}

#[tokio::test]
async fn combined_conditions_filter() {
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
                Condition::In {
                    field: "user_id".to_owned(),
                    values: vec![DbValue::from(USER_UUID), DbValue::from(OTHER_UUID)],
                },
            ]),
        )
        .unwrap();
    assert_eq!(visible_ids(&ability).await, vec![1, 2, 3]);
}

#[tokio::test]
async fn no_rules_match_nothing() {
    assert_eq!(visible_ids(&Ability::new()).await, Vec::<i64>::new());
}

#[tokio::test]
async fn manage_all_matches_everything() {
    let mut ability = Ability::new();
    ability.can(Some("manage"), Some("all")).unwrap();
    assert_eq!(visible_ids(&ability).await, vec![1, 2, 3, 4]);
}
