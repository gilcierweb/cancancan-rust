# Diesel adapter

`cancancan-diesel` renders an `Ability` as Diesel query filters - the port of
the gem's `accessible_by` for ActiveRecord. Two flavors, same rules:

- **Typed predicates** (`sqlite::accessible_by` / `postgres::accessible_by`):
  boxed Diesel expressions with bind parameters, composable with any query.
- **SQL fragment** (`accessible_by_sql`): a `WHERE` fragment with inlined
  literals, backend-agnostic, also covering `Nested` conditions.

```toml
[dependencies]
cancancan-core = "*"
cancancan-diesel = { version = "*", features = ["sqlite"] } # or "postgres"
diesel = { version = "2", features = ["sqlite"] }
```

## Setup: schema and column map

The typed path needs one `ColumnMap` per subject table, since conditions
reference columns by name at runtime:

```rust
use diesel::prelude::*;

diesel::table! {
    posts (id) {
        id -> Integer,
        user_id -> BigInt,
        published -> Bool,
    }
}

use cancancan_diesel::{ColumnMap, ColumnType};

fn post_columns() -> ColumnMap {
    ColumnMap::from([
        ("user_id".to_owned(), ColumnType::BigInt),
        ("published".to_owned(), ColumnType::Bool),
    ])
}
```

## Typed `accessible_by`

```rust
use cancancan_diesel::sqlite::accessible_by;

fn readable_posts(
    conn: &mut SqliteConnection,
    ability: &Ability,
) -> QueryResult<Vec<Post>> {
    let predicate = accessible_by::<posts::table>(
        ability, "read", "Post", "posts", &post_columns(),
    )?; // rules that cannot translate surface as CanCanError
    posts::table.filter(predicate).load(conn)
}
```

`can` rules OR together, `cannot` rules AND NOT, and with no `can` rule the
predicate matches nothing. Values travel as bind parameters, exactly like
hand-written Diesel filters.

## Fragment `accessible_by_sql`

For backends without a typed module, for logging, or for `Nested` conditions
(you declare the joins, the fragment references the relation table):

```rust
use cancancan_diesel::accessible_by_sql;
use diesel::dsl::sql;
use diesel::sql_types::Bool;

let fragment = accessible_by_sql(&ability, "read", "Post", "posts")?;
let rows = posts::table
    .filter(sql::<Bool>(&fragment))
    .load::<Post>(conn)?;
```

`condition_sql(ability, action, subject, table)?` is the single-rule version.

## Errors

- `CanCanError::AttributeArgument` - field not in the `ColumnMap`, or a value
  that does not fit the declared column type.
- `CanCanError::WrongAssociation` - `Nested` on the typed path (use the
  fragment path, with your own joins).
- `CanCanError::BlockInQuery` - a matcher rule reached the query layer; keep
  matchers for in-memory checks only.

See [query adapters](./query-adapters.md) for the shared semantics and
[rules compression](./rules_compression.md) for the automatic rule-list
optimization applied before translation.
