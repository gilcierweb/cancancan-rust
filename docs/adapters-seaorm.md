# SeaORM adapter

`cancancan-seaorm` renders an `Ability` as a `sea-query` `Condition`, accepted
by SeaORM through `QueryFilter::filter` / `SelectStatement::cond_where` - the
`accessible_by` port for SeaORM.

```toml
[dependencies]
cancancan-core = "*"
cancancan-seaorm = "*"
sea-orm = { version = "1", features = ["sqlx-sqlite", "runtime-tokio"] }
```

(The adapter depends on `sea-query` directly, so it coexists with Diesel's
bundled SQLite without symbol conflicts.)

## Setup: entity and column map

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i64,
    pub published: bool,
}

use cancancan_seaorm::{ColumnMap, ColumnType};

fn post_columns() -> ColumnMap {
    ColumnMap::from([
        ("user_id".to_owned(), ColumnType::BigInt),
        ("published".to_owned(), ColumnType::Bool),
    ])
}
```

## Filtering queries

```rust
use cancancan_seaorm::accessible_by;
use sea_orm::{EntityTrait, QueryFilter};

async fn readable_posts(
    db: &DatabaseConnection,
    ability: &Ability,
) -> Result<Vec<posts::Model>, DbErr> {
    let cond = accessible_by(ability, "read", "Post", "posts", &post_columns())?;
    posts::Entity::find().filter(cond).all(db).await
}
```

The returned `Condition` plugs into any SeaORM statement, not just selects:

```rust
// mass update only the rows the user may update
posts::Entity::update_many()
    .col_expr(posts::Column::Published, Expr::value(true))
    .filter(accessible_by(ability, "update", "Post", "posts", &post_columns())?)
    .exec(db)
    .await?;

// delete only what the ability allows
posts::Entity::delete_many()
    .filter(accessible_by(ability, "destroy", "Post", "posts", &post_columns())?)
    .exec(db)
    .await?;
```

Composition mirrors the gem: `can` predicates join with `OR`, `cannot`
predicates with `AND NOT`, and a rule set without any `can` renders a
condition matching nothing.

## Unsupported in queries

- `Nested` conditions raise `CanCanError::WrongAssociation` (join metadata
  cannot be inferred); use a [`RawSql` condition](./hash_of_conditions.md#raw-sql-fragments)
  over a query that declares its own joins.
- Matcher rules raise `CanCanError::BlockInQuery`; keep them for in-memory
  checks.
- Unknown fields raise `CanCanError::AttributeArgument`.

See [query adapters](./query-adapters.md) for the shared contract.
