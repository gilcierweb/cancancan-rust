# Query adapters

When you have an [`Ability`](../crates/cancancan-core/src/ability.rs),
adapters translate it into the filter expression your database layer expects.
The logic is shared: all adapters normalize rules the same way through `rules_for_query` and raise in the same cases:
matcher rules and `Nested` conditions are rejected (`BlockInQuery`, `WrongAssociation`), and so are unknown fields (`AttributeArgument`).

## What all adapters share

- Allow rules become an OR-based predicate (`a OR b`).
- Deny rules (`cannot`) become an OR-based negation (`AND NOT (c OR d)`).
- With no allow rule at all, the filter matches nothing (`1 = 0` / `$expr: false`).
- A match-all rule (`Condition::All`) matches everything (`1 = 1` / `{}`).
- Fields not declared in the adapter's column map raise `AttributeArgument`.
- `Nested` conditions raise `WrongAssociation` on SQL adapters (Mongo supports them).
- Matcher rules raise `BlockInQuery` for query contexts; keep them for
  controller-level checks only.

## Using an adapter

```rust
// Diesel (typed; requires feature `sqlite` or `postgres`)
use cancancan_diesel::sqlite::accessible_by;

let columns: cancancan_diesel::ColumnMap = std::collections::HashMap::from([
    ("user_id".to_owned(), cancancan_diesel::ColumnType::BigInt),
    ("published".to_owned(), cancancan_diesel::ColumnType::Bool),
]);
let predicate = accessible_by::<posts::table>(&ability, "read", "Post", "posts", &columns)?;
posts::table.filter(predicate).load(connection)?;

// Diesel (fragment style - no typed predicate, works for every backend)
use cancancan_diesel::{accessible_by_sql, condition_sql};
let fragment = accessible_by_sql(&ability, "read", "Post", "posts")?;
posts::table.filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&fragment)).load(connection)?;

// SeaORM - "easy compile with QueryFilter integration" [via Condition]
use cancancan_seaorm::{accessible_by as sea_accessible_by, condition_to_sea, ColumnMap, ColumnType};

let cond = sea_accessible_by(&ability, "read", "Post", "posts", &columns)?;
Post::find().filter(cond).all(db).await?;

// SQLx - composing into a sqlx QueryBuilder with placeholders
use cancancan_sqlx::{accessible_by as sqlx_accessible_by, ColumnMap as SqlxColumnMap, ColumnType as SqlxColumnType};
use sqlx::QueryBuilder;

let mut qb = QueryBuilder::<sqlx::Sqlite>::new("SELECT id FROM posts WHERE ");
sqlx_accessible_by(&mut qb, &ability, "read", "Post", "posts", &columns)?;

// MongoDB - filter is a root Document to pass directly to `find`, `count_documents`, etc.
use cancancan_mongo::{accessible_by as mongo_accessible_by, ColumnMap as MongoColumnMap, ColumnType as MongoColumnType};
let columns: MongoColumnMap = std::collections::HashMap::from([
    ("user_id".to_owned(), MongoColumnType::Uuid),
    ("published".to_owned(), MongoColumnType::Bool),
]);
let filter = mongo_accessible_by(&ability, "read", "Post", &columns)?;
let cursor = posts.find(filter).await?;
```

Specific notes appear in the guide of your adapter. All reject blockers
uniformly; connector variants (SQLx vs Diesel etc.) change binds only to your
backend's mobility.
