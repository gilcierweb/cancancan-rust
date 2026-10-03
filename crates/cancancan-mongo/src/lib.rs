//! Query filtering for MongoDB, mirroring `accessible_by` from the Ruby gem.
//!
//! Renders an [`Ability`] as a `bson` filter
//! [`Document`], accepted by `mongodb::Collection::find`,
//! `find_one`, `delete_many`, `update_many` and `count_documents`:
//!
//! ```rust
//! use cancancan_core::Ability;
//! use cancancan_mongo::{accessible_by, ColumnMap, ColumnType};
//!
//! # fn filter(ability: &Ability) -> Result<bson::Document, cancancan_mongo::CanCanError> {
//! let columns: ColumnMap = [("user_id".to_owned(), ColumnType::ObjectId)].into();
//! let filter = accessible_by(ability, "read", "Post", &columns)?;
//! Ok(filter)
//! # }
//! ```
//!
//! Mapping notes specific to MongoDB semantics:
//!
//! - Nested conditions flatten into dot-notation (`author.name`), which is how
//!   MongoDB naturally models relations inside documents.
//! - `Condition::Not` renders as `$nor` (De Morgan safe against `$ne null`
//!   semantics).
//! - A rule set with no `can` predicate renders `$expr: false` (matches
//!   nothing; requires server 3.6+); a catch-all rule renders the empty
//!   document (matches all).
//! - `Condition::RawSql` mirrors the gem SQL escape hatch and has no MongoDB
//!   mapping, so it surfaces as [`CanCanError::RawSqlNotSupported`].
//!
//! # Semantic choices (where MongoDB absent-diverges from SQL)
//!
//! - `IS NULL` (`Eq`/`IsNull` with `Null`) renders `{ field: { $type: "null" } }`:
//!   only documents whose field exists with a real null. `IS NOT NULL` renders
//!   `{ field: { $exists: true, $ne: null } }`: only documents with a present,
//!   non-null value. Missing fields match neither, unlike loose
//!   `{ field: null }` (which matches missing too).
//! - [`ColumnType::Uuid`] binds native BSON binary UUID (subtype 4,
//!   `UuidRepresentation::Standard`) — matching what `mongodb`/`bson`
//!   serialize by default. String-stored UUIDs no longer match; declare the
//!   field `Text` instead.
//! - `ColumnMap` is keyed by leaf field name regardless of nesting depth.

use std::collections::HashMap;

use bson::{Document, oid::ObjectId, uuid::UuidRepresentation};

pub use cancancan_core::{Ability, CanCanError, Condition, DbValue};

/// Column type backing a condition field for MongoDB binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnType {
    /// 32-bit integer (`Bson::Int32`, Rust `i32`).
    Int,
    /// 64-bit integer (`Bson::Int64`, Rust `i64`).
    Long,
    /// Double precision float (`Bson::Double`, Rust `f64`).
    Double,
    /// Text (`Bson::String`, Rust `String`).
    Text,
    /// Boolean (`Bson::Boolean`, Rust `bool`).
    Bool,
    /// Object identifier (`Bson::ObjectId`), validated 24-hex strings.
    ObjectId,
    /// UUID stored as text (`uuid::Uuid` canonical form).
    Uuid,
    /// JSON payload stored as text.
    Json,
}

/// Maps field names to their column types for one subject collection.
pub type ColumnMap = HashMap<String, ColumnType>;

/// Renders `condition` as a MongoDB filter document.
///
/// # Errors
///
/// Returns [`CanCanError::AttributeArgument`] for unknown fields, invalid
/// identifiers and value/type mismatches,
/// [`CanCanError::WrongAssociation`] for invalid relation names, and
/// [`CanCanError::RawSqlNotSupported`] for raw SQL conditions.
pub fn condition_to_doc(
    condition: &Condition,
    columns: &ColumnMap,
) -> Result<Document, CanCanError> {
    render_condition(condition, columns, &[])
}

/// Renders the filter selecting every record `action` may access.
///
/// `can` conditions compose with `$or`, `cannot` conditions with `$nor`,
/// mirroring the query composition of the Ruby gem. With no relevant `can`
/// rule the filter matches nothing (`$expr: false`); a catch-all rule
/// renders the empty document.
///
/// # Errors
///
/// Same as [`condition_to_doc`], plus [`CanCanError::BlockInQuery`] for
/// matcher rules through [`Ability::rules_for_query`].
pub fn accessible_by(
    ability: &Ability,
    action: &str,
    subject_type: &str,
    columns: &ColumnMap,
) -> Result<Document, CanCanError> {
    let rules = ability.rules_for_query(action, subject_type)?;
    let rules = if cancancan_core::rules_compressor_enabled() {
        cancancan_core::compress(rules)
    } else {
        rules
    };

    let mut allowed: Vec<Document> = Vec::new();
    let mut denied: Vec<Document> = Vec::new();
    for rule in &rules {
        let filter = condition_to_doc(rule.condition(), columns)?;
        if rule.allows() {
            allowed.push(filter);
        } else {
            denied.push(filter);
        }
    }

    let mut filter = Document::new();
    if allowed.is_empty() {
        return Ok(bson::doc! { "$expr": false });
    }
    if !(allowed.len() == 1 && allowed[0] == Document::new()) {
        push_or_group(&mut filter, allowed);
    }
    if !denied.is_empty() {
        let denied_docs: Vec<bson::Bson> = denied.into_iter().map(bson::Bson::Document).collect();
        filter.insert("$nor", bson::Array::from(denied_docs));
    }
    Ok(filter)
}

fn push_or_group(filter: &mut Document, mut allowed: Vec<Document>) {
    if allowed.len() == 1 {
        for (field, value) in allowed.remove(0) {
            filter.insert(field, value);
        }
        return;
    }
    let list: Vec<bson::Bson> = allowed.into_iter().map(bson::Bson::Document).collect();
    filter.insert("$or", list);
}

fn render_condition(
    condition: &Condition,
    columns: &ColumnMap,
    path: &[String],
) -> Result<Document, CanCanError> {
    match condition {
        Condition::All => Ok(Document::new()),
        Condition::Eq { field, value } => match value {
            DbValue::Null => is_null_doc(columns, path, field, true),
            DbValue::List(values) => in_doc(columns, path, field, values, false),
            _ => Ok(doc_single(
                columns,
                path,
                field,
                comparison_value(columns, field, value)?,
                false,
            )?),
        },
        Condition::Ne { field, value } => match value {
            DbValue::Null => is_null_doc(columns, path, field, false),
            DbValue::List(values) => in_doc(columns, path, field, values, true),
            _ => Ok(doc_single(
                columns,
                path,
                field,
                comparison_value(columns, field, value)?,
                true,
            )?),
        },
        Condition::In { field, values } => in_doc(columns, path, field, values, false),
        Condition::Range { field, min, max } => {
            let qualified = qualified_field(path, field)?;
            let column_type = lookup(columns, field)?;
            Ok(bson::doc! {
                qualified: {
                    "$gte": db_value_to_bson(min, column_type)?,
                    "$lte": db_value_to_bson(max, column_type)?,
                }
            })
        }
        Condition::IsNull { field, is_null } => is_null_doc(columns, path, field, *is_null),
        Condition::And(parts) => joined_doc(parts, columns, path, "$and"),
        Condition::Or(parts) => joined_doc(parts, columns, path, "$or"),
        Condition::Not(inner) => {
            let body = render_condition(inner, columns, path)?;
            Ok(bson::doc! { "$nor": [body] })
        }
        Condition::Nested {
            relation,
            condition,
        } => {
            if !cancancan_core::is_identifier(relation) {
                return Err(CanCanError::WrongAssociation(relation.clone()));
            }
            let mut deeper = path.to_vec();
            deeper.push(relation.clone());
            render_condition(condition, columns, &deeper)
        }
        Condition::RawSql(_) => Err(CanCanError::RawSqlNotSupported("mongodb")),
        _ => Err(CanCanError::AttributeArgument),
    }
}

fn joined_doc(
    parts: &[Condition],
    columns: &ColumnMap,
    path: &[String],
    group: &str,
) -> Result<Document, CanCanError> {
    let mut list: Vec<bson::Bson> = Vec::with_capacity(parts.len());
    for part in parts {
        list.push(bson::Bson::Document(render_condition(part, columns, path)?));
    }
    if list.is_empty() {
        return Ok(if group == "$and" {
            Document::new()
        } else {
            bson::doc! { "$expr": false }
        });
    }
    if list.len() == 1 {
        let first = list.remove(0);
        let bson::Bson::Document(doc) = first else {
            return Ok(bson::doc! {});
        };
        return Ok(doc);
    }
    Ok(bson::doc! { group: list })
}

fn doc_single(
    columns: &ColumnMap,
    path: &[String],
    field: &str,
    value: bson::Bson,
    negated: bool,
) -> Result<Document, CanCanError> {
    lookup(columns, field)?;
    let qualified = qualified_field(path, field)?;
    if negated {
        Ok(bson::doc! { qualified: { "$ne": value } })
    } else {
        Ok(bson::doc! { qualified: value })
    }
}

fn comparison_value(
    columns: &ColumnMap,
    field: &str,
    value: &DbValue,
) -> Result<bson::Bson, CanCanError> {
    let column_type = lookup(columns, field)?;
    db_value_to_bson(value, column_type)
}

fn in_doc(
    columns: &ColumnMap,
    path: &[String],
    field: &str,
    values: &[DbValue],
    negated: bool,
) -> Result<Document, CanCanError> {
    let qualified = qualified_field(path, field)?;
    let column_type = lookup(columns, field)?;
    let mut bound: Vec<bson::Bson> = Vec::with_capacity(values.len());
    for value in values {
        bound.push(db_value_to_bson(value, column_type)?);
    }
    let operator = if negated { "$nin" } else { "$in" };
    Ok(bson::doc! { qualified: { operator: bound } })
}

fn is_null_doc(
    columns: &ColumnMap,
    path: &[String],
    field: &str,
    is_null: bool,
) -> Result<Document, CanCanError> {
    lookup(columns, field)?;
    let qualified = qualified_field(path, field)?;
    if is_null {
        // SQL is faithful: only documents whose field exists with BSON type
        // Null. (Loose `{ field: null }` would also match missing fields.)
        Ok(bson::doc! { qualified: { "$type": "null" } })
    } else {
        // strict NOT NULL: field exists and holds a non-null value
        Ok(bson::doc! { qualified: { "$exists": true, "$ne": bson::Bson::Null } })
    }
}

fn qualified_field(path: &[String], field: &str) -> Result<String, CanCanError> {
    if !cancancan_core::is_identifier(field) {
        return Err(CanCanError::AttributeArgument);
    }
    let mut qualified = path.to_vec();
    qualified.push(field.to_owned());
    Ok(qualified.join("."))
}

fn lookup(columns: &ColumnMap, field: &str) -> Result<ColumnType, CanCanError> {
    columns
        .get(field)
        .copied()
        .ok_or(CanCanError::AttributeArgument)
}

fn db_value_to_bson(value: &DbValue, column_type: ColumnType) -> Result<bson::Bson, CanCanError> {
    match value {
        DbValue::Bool(flag) => match column_type {
            ColumnType::Bool => Ok(bson::Bson::Boolean(*flag)),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Int(number) => match column_type {
            ColumnType::Int => i32::try_from(*number)
                .map(bson::Bson::Int32)
                .map_err(|_| CanCanError::AttributeArgument),
            ColumnType::Long => Ok(bson::Bson::Int64(*number)),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Float(number) => match column_type {
            ColumnType::Double => Ok(bson::Bson::Double(*number)),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Str(text) => match column_type {
            ColumnType::Text | ColumnType::Json => Ok(bson::Bson::String(text.clone())),
            ColumnType::ObjectId => {
                let parsed =
                    ObjectId::parse_str(text).map_err(|_| CanCanError::AttributeArgument)?;
                Ok(bson::Bson::ObjectId(parsed))
            }
            ColumnType::Uuid => uuid::Uuid::parse_str(text)
                .map(|parsed| {
                    bson::Bson::Binary(bson::Binary::from_uuid_with_representation(
                        parsed.into(),
                        UuidRepresentation::Standard,
                    ))
                })
                .map_err(|_| CanCanError::AttributeArgument),
            _ => Err(CanCanError::AttributeArgument),
        },
        _ => Err(CanCanError::AttributeArgument),
    }
}
