//! Typed `accessible_by` for SQLite.
//!
//! Renders an [`Ability`](cancancan_core::Ability) as a boxed Diesel
//! predicate with bind parameters, applied through any query's `filter`:
//!
//! ```rust,no_run
//! use cancancan_core::Ability;
//! use cancancan_diesel::sqlite::accessible_by;
//! use cancancan_diesel::{ColumnMap, ColumnType};
//! use diesel::prelude::*;
//! use std::collections::HashMap;
//!
//! # table! { posts (id) { id -> Integer, user_id -> Integer, } }
//! # fn load(
//! #     connection: &mut SqliteConnection,
//! #     ability: &Ability,
//! # ) -> QueryResult<Vec<(i32, i32)>> {
//! let columns: ColumnMap = HashMap::from([
//!     ("id".to_owned(), ColumnType::Integer),
//!     ("user_id".to_owned(), ColumnType::Integer),
//! ]);
//! let predicate =
//!     accessible_by::<posts::table>(ability, "read", "Post", "posts", &columns)
//!         .expect("rules translate to SQL");
//! posts::table
//!     .select((posts::id, posts::user_id))
//!     .filter(predicate)
//!     .load(connection)
//! # }
//! ```
//!
//! The same predicate scopes updates and deletes:
//! `diesel::update(posts::table.filter(predicate))`.

use crate::typed::backend_predicates;

backend_predicates!(diesel::sqlite::Sqlite);
