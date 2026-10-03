# MongoDB adapter

`cancancan-mongo` renders an `Ability` as a `bson::Document` filter, accepted
directly by `mongodb::Collection::find`, `find_one`, `update_many`,
`delete_many` and `count_documents`. The crate depends only on `bson` - bring
your own `mongodb` client.

```toml
[dependencies]
cancancan-core = "*"
cancancan-mongo = "*"
mongodb = "3"
bson = { version = "2", features = ["uuid-1"] }
```

## Setup: column map

```rust
use cancancan_mongo::{ColumnMap, ColumnType};

fn post_columns() -> ColumnMap {
    ColumnMap::from([
        ("user_id".to_owned(), ColumnType::Uuid),
        ("published".to_owned(), ColumnType::Bool),
    ])
}
```

## Filtering

```rust
use cancancan_mongo::accessible_by;

async fn readable_posts(
    collection: &mongodb::Collection<Post>,
    ability: &Ability,
) -> mongodb::error::Result<Vec<Post>> {
    let filter = accessible_by(ability, "read", "Post", &post_columns())?;
    let mut cursor = collection.find(filter).await?;
    cursor.try_collect().await
}
```

No table name argument: MongoDB filters are collection-scoped already, so the
call is `accessible_by(ability, action, subject_type, &columns)`.

Composition mirrors the gem: `can` conditions OR together (`$or`), `cannot`
conditions negate (`$nor` / `$and` with negation). A rule set with no `can`
predicate renders `{ $expr: false }` (matches nothing, server 3.6+); a
catch-all rule renders the empty document (matches all).

## MongoDB-specific semantics

- **`Nested` conditions flatten into dot notation** - `author.name` - which is
  MongoDB's natural way to model relations inside documents. No joins needed,
  unlike the SQL adapters.
- **`Condition::Not` renders as `$nor`**, which is De Morgan safe against
  `$ne: null` semantics.
- **Null handling is strict:**
  - `IsNull { is_null: true }` (or `Eq` with `DbValue::Null`) renders
    `{ field: { $type: "null" } }` - only documents where the field exists
    with a real null.
  - `IsNull { is_null: false }` renders `{ field: { $exists: true, $ne: null } }`.
  - Missing fields match neither, unlike a loose `{ field: null }` which
    would match missing too.
- **`ColumnType::Uuid` binds native BSON binary UUID** (subtype 4,
  `UuidRepresentation::Standard`) - matching what `mongodb`/`bson` serialize
  by default. If your UUIDs are stored as strings, declare the field `Text`.
- **`RawSql` has no MongoDB mapping** and raises
  `CanCanError::RawSqlNotSupported`; express the predicate as a structured
  condition instead.

## Errors

- `CanCanError::AttributeArgument` - undeclared field or value/type mismatch.
- `CanCanError::BlockInQuery` - matcher rules cannot become a filter.
- `CanCanError::RawSqlNotSupported` - raw SQL on the NoSQL backend.

See [query adapters](./query-adapters.md) for the shared contract.
