//! Typed `accessible_by` for PostgreSQL.
//!
//! Renders an [`Ability`](cancancan_core::Ability) as a boxed Diesel
//! predicate with bind parameters, applied through any query's `filter`.
//! See [`crate::sqlite`] for the usage pattern; only the backend differs.

use crate::typed::backend_predicates;

backend_predicates!(diesel::pg::Pg; uuid: (diesel::sql_types::Uuid, crate::typed::as_uuid_native));
