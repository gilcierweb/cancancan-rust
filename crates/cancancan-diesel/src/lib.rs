//! Query filtering for Diesel, mirroring `accessible_by` from the Ruby gem.
//!
//! Two flavors, same rules:
//!
//! * Typed predicates (`sqlite::accessible_by`, `postgres::accessible_by`):
//!   boxed Diesel expressions with bind parameters, composable with any
//!   query builder call. Needs one `ColumnMap` (crate root) per subject
//!   table and the matching backend feature.
//! * [`accessible_by_sql`]: a SQL `WHERE` fragment with inlined literals,
//!   backend-agnostic, also covering joined associations and raw SQL
//!   inspection. Prefer it for logging and for cases the typed path rejects.
//!
//! Gem to Rust mapping:
//!
//! | Ruby gem                      | This crate                                |
//! |-------------------------------|-------------------------------------------|
//! | `Post.accessible_by(ability)` | `posts::table.filter(accessible_by(..)?)` |
//! | hash conditions               | [`cancancan_core::Condition`]             |
//! | block `can` in `accessible_by` | [`CanCanError::BlockInQuery`]           |
//! | `cannot` with attributes      | rejected, like `relevant_rules_for_query` |
//!
//! Not ported yet: join strategies (`left_join`/`subquery`/`exists` from the
//! gem adapters) for nested association conditions. `Nested` renders through
//! [`accessible_by_sql`] (caller joins first) and errors in the typed path.
//!
//! # Example
//!
//! ```rust,no_run
//! use cancancan_core::Ability;
//! use cancancan_diesel::accessible_by_sql;
//! use diesel::dsl::sql;
//! use diesel::prelude::*;
//! use diesel::sql_types::Bool;
//!
//! # table! { posts (id) { id -> Integer, user_id -> Integer, } }
//! # fn load(
//! #     connection: &mut SqliteConnection,
//! #     ability: &Ability,
//! # ) -> QueryResult<Vec<(i32, i32)>> {
//! let fragment = accessible_by_sql(ability, "read", "Post", "posts")
//!     .expect("rules translate to SQL");
//! posts::table
//!     .select((posts::id, posts::user_id))
//!     .filter(sql::<Bool>(&fragment))
//!     .load(connection)
//! # }
//! ```

mod fragment;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
mod typed;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use cancancan_core::{Ability, CanCanError, Condition, DbValue};
pub use fragment::{accessible_by_sql, condition_sql};
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub use typed::{ColumnMap, ColumnType};
