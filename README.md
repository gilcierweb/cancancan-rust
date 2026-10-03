# cancancan-rust

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)
![Edition](https://img.shields.io/badge/edition-2024-red.svg)
![Clippy](https://img.shields.io/badge/clippy-all%20%2B%20pedantic-green.svg)

A declarative authorization library for Rust, inspired by the Ruby
[CanCanCan](https://github.com/CanCanCommunity/cancancan) gem. Define `allow`/`deny`
rules in a central `Ability`, check them against subject types or concrete instances,
and translate those same rules into database queries so authorization and data access
never drift apart.

## Overview

In CanCanCan, every permission lives in a single `Ability` object: `can :read, Post`.
This project ports that model to Rust with a zero-cost, backend-agnostic core and a
set of adapter crates that cover the two most popular web frameworks and the four
most popular persistence layers.

Core semantics, straight from the gem:

- Rules are ordered and **the last matching rule wins** (`can` then `cannot` overrides).
- `"manage"` matches every action and `"all"` matches every subject - the gem's
  `:manage` and `:all` pseudo-actions.
- Checks answer `can?`/`cannot?`; `authorize!` fails with a structured
  `AccessDenied` error carrying a resolvable message.
- `alias_action` groups actions (defaults included) under a single alias.
- Web adapters mirror `controller_additions`: a `current_ability` extractor, a
  `check_authorization` middleware that fails handlers that never authorize, and
  `skip_authorization_check` to opt out.
- Query adapters mirror `accessible_by`: the same rules that guard a route also
  become the `WHERE` clause of your list queries.

## Features

- **Central rule engine** - `Ability` with `can`/`cannot` plus `_where` (hash
  conditions), `_matching` (block matchers) and `_attributes` (strong parameters)
  variants.
- **Rich condition tree** - `Condition` supports equality, comparison, `IN`,
  ranges, negation, nesting and a raw SQL escape hatch; `SubjectInstance` adapts
  any domain type to attribute lookups.
- **Action aliases** - `alias_action`, `aliased_actions`, `clear_aliased_actions`
  with the gem's target-validation guard.
- **Rule compressor** - optional merging of overlapping rules for query rendering
  (`set_rules_compressor_enabled`).
- **Custom denial messages** - pluggable `MessageResolver`, mirroring the gem's
  i18n lookup, with a `default_message` fallback.
- **Introspection** - `permissions`, `attributes_for`, `permitted_attributes`,
  `has_matcher`, `has_raw_sql`, `merge`.
- **Web integrations** - extractors and middleware for Axum and Actix Web, mapping
  `AccessDenied` to `403` and wiring mistakes to `500`.
- **Query adapters** - `accessible_by` for Diesel, SeaORM (sea-query), SQLx and
  MongoDB with typed, injection-safe binds.
- **Scaffolding CLI** - generate a starter ability module for new projects.

## Workspace Layout

| Crate | Purpose |
|---|---|
| [`cancancan-core`](crates/cancancan-core) | Rule engine: `Ability`, `Condition`, `Rule`, aliases, errors, messages |
| [`cancancan-axum`](crates/cancancan-axum) | `CurrentAbility` extractor, `check_authorization` middleware, HTTP error mapping |
| [`cancancan-actix`](crates/cancancan-actix) | Same integration surface for Actix Web 4 |
| [`cancancan-diesel`](crates/cancancan-diesel) | `accessible_by` as typed Diesel predicates or a backend-agnostic SQL fragment |
| [`cancancan-seaorm`](crates/cancancan-seaorm) | `accessible_by` as `sea_query::SimpleExpr` / `SeaCondition` |
| [`cancancan-sqlx`](crates/cancancan-sqlx) | `accessible_by` pushed into a `sqlx::QueryBuilder` with typed binds |
| [`cancancan-mongo`](crates/cancancan-mongo) | `accessible_by` rendered as a `bson::Document` filter |
| [`cancancan-cli`](crates/cancancan-cli) | `cancancan scaffold` command that writes a starter ability module |

## Tech Stack

- **Language**: Rust (edition 2024, MSRV 1.85)
- **Web**: [axum](https://crates.io/crates/axum) 0.8, [actix-web](https://crates.io/crates/actix-web) 4
- **SQL**: [diesel](https://crates.io/crates/diesel) 2.2 (SQLite/PostgreSQL),
  [sea-query](https://crates.io/crates/sea-query) 0.32,
  [sqlx](https://crates.io/crates/sqlx) 0.8 (PostgreSQL/MySQL/SQLite)
- **NoSQL**: [bson](https://crates.io/crates/bson) 2 / [mongodb](https://crates.io/crates/mongodb) 3
- **CLI**: [clap](https://crates.io/crates/clap) 4 (derive)
- **Errors**: [thiserror](https://crates.io/crates/thiserror) 2
- **Testing**: [rstest](https://crates.io/crates/rstest), [pretty_assertions](https://crates.io/crates/pretty_assertions),
  [testcontainers](https://crates.io/crates/testcontainers) (real `mongod` e2e)
- **Quality gates**: `clippy::all` + `clippy::pedantic` as warnings, `unsafe_code` warned workspace-wide

## Requirements

- Rust 1.85 or newer (install via [rustup](https://rustup.rs))
- Docker (only for the MongoDB end-to-end tests)

## Getting Started

Clone and build:

```bash
git clone https://github.com/GilcierWeb/cancancan-rust.git
cd cancancan-rust
cargo build --workspace
```

Run the test suite (Docker must be running for the MongoDB e2e tests):

```bash
cargo test --workspace
```

### Installation

The crates are not published to crates.io yet. Reference them by path, or by Git:

```toml
[dependencies]
cancancan-core  = { path = "vendor/cancancan-rust/crates/cancancan-core" }
cancancan-axum  = { path = "vendor/cancancan-rust/crates/cancancan-axum" }
cancancan-sqlx  = { path = "vendor/cancancan-rust/crates/cancancan-sqlx" }
```

Or via Git dependency:

```toml
[dependencies]
cancancan-core = { git = "https://github.com/GilcierWeb/cancancan-rust.git", branch = "main" }
```

## Usage

### 1. Define an ability

Implement `SubjectInstance` for your domain types, then declare ordered rules.
The last matching rule wins, exactly like the gem:

```rust
use cancancan_core::{Ability, Condition, DbValue, SubjectInstance};

struct Post { user_id: i64 }

impl SubjectInstance for Post {
    fn subject_type(&self) -> &'static str { "Post" }
    fn attribute(&self, name: &str) -> Option<DbValue> {
        match name {
            "user_id" => Some(DbValue::Int(self.user_id)),
            _ => None,
        }
    }
}

fn ability_for(user_id: i64, is_admin: bool) -> Ability {
    let mut ability = Ability::new();
    if is_admin {
        ability.can(Some("manage"), Some("all")).unwrap(); // can :manage, :all
    } else {
        ability.can_where(Some("read"), Some("Post"), Condition::Eq {
            field: "user_id".to_owned(),
            value: DbValue::Int(user_id),
        }).unwrap();
    }
    ability
}
```

### 2. Check and enforce

```rust
let ability = ability_for(1, false);

ability.can_check("read", &Post { user_id: 1 });     // true  (can?)
ability.cannot_check("read", &Post { user_id: 2 }); // true  (cannot?)
ability.authorize("read", &Post { user_id: 2 })?;    // Err(AccessDenied) (authorize!)
```

### 3. Authorize requests - Axum

Provide the request-scoped ability via extensions (from your auth middleware), use
the `CurrentAbility` extractor in handlers, and register `check_authorization` to
catch handlers that never authorize:

```rust
use std::sync::Arc;
use axum::{middleware, routing::get, Extension, Router};
use cancancan_axum::{check_authorization, AuthorizationError, CurrentAbility};

async fn show(ability: CurrentAbility) -> Result<&'static str, AuthorizationError> {
    ability.authorize("read", &Post { user_id: 1 })?; // 403 on denial
    Ok("show")
}

async fn health(ability: CurrentAbility) -> &'static str {
    ability.skip_authorization_check(); // skip_authorization_check
    "ok"
}

let app = Router::new()
    .route("/posts/{id}", get(show))
    .route("/health", get(health))
    .layer(middleware::from_fn(check_authorization)) // 500 when a handler never authorized
    .layer(Extension(Arc::new(ability_for(1, false))));
```

`AccessDenied` maps to `403 Forbidden`; any other authorization error (including
a missing ability in extensions) maps to `500`.

### 4. Authorize requests - Actix Web

The same surface for Actix: register the ability as app data, extract
`CurrentAbility`, wrap the app with `check_authorization`:

```rust
use actix_web::{middleware, web, App, HttpServer};
use cancancan_actix::{check_authorization, CurrentAbility};

async fn show(ability: CurrentAbility) -> Result<&'static str, actix_web::Error> {
    ability.authorize("read", &Post { user_id: 1 })?;
    Ok("show")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(ability_for(1, false)))
            .route("/posts/{id}", web::get().to(show))
            .wrap(middleware::from_fn(check_authorization))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

### 5. Scope queries - `accessible_by`

The same rules that guard routes also filter queries, so listing endpoints only
return records the user may read.

**SQLx** (PostgreSQL/MySQL/SQLite) - pushed into a `QueryBuilder` with typed,
injection-safe binds:

```rust
use cancancan_sqlx::{accessible_by, ColumnMap, ColumnType};
use sqlx::QueryBuilder;

let columns: ColumnMap = [("user_id".to_owned(), ColumnType::BigInt)].into();
let mut qb = QueryBuilder::<sqlx::Sqlite>::new("SELECT id FROM posts WHERE ");
accessible_by(&mut qb, &ability, "read", "Post", "posts", &columns)?;
let rows = qb.build_query_scalar::<i64>().fetch_all(&pool).await?;
```

**Diesel** - typed predicates per backend (enable the `sqlite` or `postgres`
feature) or a backend-agnostic SQL fragment:

```rust
use cancancan_diesel::accessible_by_sql;
use diesel::{dsl::sql, prelude::*, sql_types::Bool};

// Backend-agnostic fragment (inlined literals, works for logging too):
let fragment = accessible_by_sql(&ability, "read", "Post", "posts")?;
posts::table
    .filter(sql::<Bool>(&fragment))
    .load(connection)?;
```

**SeaORM** (sea-query) - returned as a `SeaCondition`:

```rust
use cancancan_seaorm::accessible_by;

let condition = accessible_by(&ability, "read", "Post", "posts", &columns)?;
let posts = Post::find()
    .filter(condition)
    .all(&db)
    .await?;
```

**MongoDB** - returned as a `bson::Document` filter for
`find`/`find_one`/`delete_many`/`update_many`/`count_documents`:

```rust
use cancancan_mongo::{accessible_by, ColumnMap, ColumnType};

let columns: ColumnMap = [("user_id".to_owned(), ColumnType::Long)].into();
let filter = accessible_by(&ability, "read", "Post", &columns)?;
let cursor = collection.find(filter).await?;
```

### 6. Scaffold a new ability module

```bash
cargo run -p cancancan-cli -- scaffold --output src/ability.rs
```

## API Mapping - CanCanCan Gem to Rust

| Ruby gem                | Rust equivalent |
|-------------------------|----------------------------------------------|
| `can` / `cannot`        | `Ability::can` / `Ability::cannot` (+ `can_where`, `can_matching`, `can_attributes` and `cannot_*` variants) |
| `can?` / `cannot?`      | `Ability::can_check` / `Ability::cannot_check` (`can_check_type` for class-level checks) |
| `authorize!`            | `Ability::authorize` / `Ability::authorize_type` |
| `alias_action`          | `Ability::alias_action` |
| `aliased_actions`       | `Ability::aliased_actions` |
| `clear_aliased_actions` | `Ability::clear_aliased_actions` |
| `merge`                 | `Ability::merge` |
| `permissions`           | `Ability::permissions` |
| `attributes_for`        | `Ability::attributes_for` |
| `permitted_attributes`  | `Ability::permitted_attributes` |
| `has_block?`            | `Ability::has_matcher` |
| `has_raw_sql?`          | `Ability::has_raw_sql` |
| hash conditions         | `Condition` |
| `:manage` / `:all`      | `"manage"` / `"all"` strings |
| `current_ability`       | `CurrentAbility` extractor (web crates) |
| `check_authorization`   | `check_authorization` middleware (web crates) |
| `Model.accessible_by`   | `accessible_by` (adapter crates) |

Translation conventions where Rust cannot spell the gem: the `?` suffix becomes
`_check`, the `!` suffix is dropped (`authorize!` pairs with the `?` operator),
and `Error` suffixes on error variants are dropped (the enum is the error).

## Development

```bash
# Build everything
cargo build --workspace

# Run the test suite (MongoDB e2e tests need a running Docker daemon)
cargo test --workspace

# Lint with the workspace gates (clippy::all + clippy::pedantic)
cargo clippy --workspace --all-targets

# Format
cargo fmt --all
```

To skip the Docker-backed MongoDB e2e suite:

```bash
cargo test --workspace --exclude cancancan-mongo
```

## Contributing

Contributions are welcome! Feel free to open issues and pull requests.

1. Keep changes inside `cancancan-rust/`; commit only from this directory.
2. Mirror the gem's semantics; cite the source file/line in comments when
   porting behavior.
3. Add tests first (both sync and async where applicable).
4. Ensure the quality gates pass before suggesting a PR.
5. Target Conventional Commits titles in English.

## Inspiration

Inspired by the legendary [Ruby CanCanCan gem](https://github.com/CanCanCommunity/cancancan). CanCanCan-Rust brings its intuitive,
battle-tested authorization model to the Rust ecosystem - with Rust's
performance, safety and compile-time guarantees.

## License

Released under the [MIT License](LICENSE).

### Links
- Repository: https://github.com/gilcierweb/cancancan-rust

### Author
Built and maintained by [GilcierWeb](https://gilcierweb.com.br).