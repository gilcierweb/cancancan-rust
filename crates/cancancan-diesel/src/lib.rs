//! Query filtering for Diesel, mirroring `accessible_by` from the Ruby gem.
//!
//! [`accessible_by_sql`] renders an [`Ability`](cancancan_core::Ability) as a
//! SQL `WHERE` fragment with inlined literals, applied through
//! `diesel::dsl::sql`. Values are escaped, identifiers validated and quoted,
//! so the fragment runs on PostgreSQL, MySQL and SQLite.
//!
//! # Example
//!
//! ```rust,no_run
//! use cancancan_core::{Ability, Condition, DbValue};
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

pub use cancancan_core::{Ability, CanCanError, Condition, DbValue};
pub use fragment::{accessible_by_sql, condition_sql};
