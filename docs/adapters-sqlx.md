# SQLx adapter

`cancancan-sqlx` pushes an `Ability` into a `sqlx::QueryBuilder` as `WHERE`
conditions with real bind parameters - `accessible_by` for projects that build
SQL by hand. Placeholders are numbered per backend: `$N` for Postgres, `?` for
MySQL and SQLite.

```toml
[dependencies]
cancancan-core = "*"
cancancan-sqlx = "*"
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
```

## Setup: column map

Conditions reference columns by name, so declare one `ColumnMap` per subject
table:

```rust
use cancancan_sqlx::{ColumnMap, ColumnType};

fn post_columns() -> ColumnMap {
    ColumnMap::from([
        ("user_id".to_owned(), ColumnType::BigInt),
        ("published".to_owned(), ColumnType::Bool),
    ])
}
```

## Building a query

```rust
use cancancan_sqlx::accessible_by;
use sqlx::QueryBuilder;

async fn readable_posts(
    pool: &sqlx::SqlitePool,
    ability: &Ability,
) -> Result<Vec<(i64,)>, sqlx::Error> {
    let mut qb = QueryBuilder::<sqlx::Sqlite>::new("SELECT id FROM posts WHERE ");
    accessible_by(&mut qb, ability, "read", "Post", "posts", &post_columns())?;
    qb.build_query_scalar::<i64>().fetch_all(pool).await
}
```

Identifiers render quoted as `"table"."field"`; values always bind, nothing
is inlined.

The builder is not restricted to `SELECT` - the same pattern works for
`UPDATE ... WHERE` and `DELETE ... WHERE`, enforcing permissions on mass
operations:

```rust
let mut qb = QueryBuilder::<sqlx::Sqlite>::new("DELETE FROM posts WHERE ");
accessible_by(&mut qb, ability, "destroy", "Post", "posts", &post_columns())?;
qb.build().execute(pool).await?;
```

## Composition rules

Same as every adapter:

- `can` predicates join with `OR`
- `cannot` predicates with `AND NOT (...)`
- no `can` rule: the condition matches nothing
- `Condition::All`: matches everything
- `RawSql` fragments pass through inline (they are your responsibility)

## Errors

- `CanCanError::AttributeArgument` - undeclared field or value/type mismatch.
- `CanCanError::WrongAssociation` - `Nested` condition; write the join
  yourself and use `RawSql` for the predicate.
- `CanCanError::BlockInQuery` - matcher rule reached the query layer.

UUID columns (`ColumnType::Uuid`) are validated with `uuid::Uuid::parse_str`
and bind natively with the `sqlx/uuid` feature; without it they bind as text,
which works on MySQL/SQLite and on Postgres through an explicit cast.

See [query adapters](./query-adapters.md) for the shared contract.
