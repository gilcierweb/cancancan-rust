//! Query filtering for `sqlx`, mirroring `accessible_by` from the Ruby gem.
//!
//! Pushes an [`Ability`](cancancan_core::Ability) into a
//! [`sqlx::QueryBuilder`] as `WHERE` conditions with bind parameters
//! (placeholders are numbered per backend: `$N` for Postgres, `?` for
//! MySQL/SQLite):
//!
//! ```rust
//! use cancancan_core::{Ability, Condition, DbValue};
//! use cancancan_sqlx::{accessible_by, ColumnMap, ColumnType};
//! use sqlx::QueryBuilder;
//!
//! # fn build(ability: &Ability) -> Result<(), cancancan_sqlx::CanCanError> {
//! let columns: ColumnMap = [("user_id".to_owned(), ColumnType::Uuid)].into();
//! let mut qb = QueryBuilder::<sqlx::Sqlite>::new("SELECT id FROM posts WHERE ");
//! accessible_by(&mut qb, ability, "read", "Post", "posts", &columns)?;
//! let rows = qb.build_query_scalar::<i64>();
//! # Ok(())
//! # }
//! ```
//!
//! Identifier columns are validated and rendered `"table"."field"`.
//! UUID columns accept UUID-formatted strings (validated via
//! [`uuid::Uuid::parse_str`]); without the `sqlx/uuid` feature they bind as
//! text, which works on MySQL and SQLite and on Postgres when the column is
//! compared through a cast.

use sqlx::{Database, Encode, QueryBuilder, Type};
use std::collections::HashMap;

pub use cancancan_core::{Ability, CanCanError, Condition, DbValue};

/// SQL column type backing a condition field for `sqlx` binds.
///
/// The declared type selects the bind representation (and range/validation
/// checks), since queries reference columns dynamically by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnType {
    /// 16-bit integer (Rust `i16`).
    SmallInt,
    /// 32-bit integer (Rust `i32`).
    Integer,
    /// 64-bit integer (Rust `i64`).
    BigInt,
    /// Single precision float (Rust `f32`).
    Float,
    /// Double precision float (Rust `f64`).
    Double,
    /// Text (Rust `String`).
    Text,
    /// Boolean (Rust `bool`).
    Bool,
    /// UUID column, validated then bound as text.
    Uuid,
    /// JSON column, bound as text.
    Json,
}

/// Maps field names to their column types for one subject table.
pub type ColumnMap = HashMap<String, ColumnType>;

/// Pushes `condition` as SQL into `builder` with typed binds.
///
/// # Errors
///
/// Returns [`CanCanError::AttributeArgument`] for unknown fields, invalid
/// identifiers and value/type mismatches, and
/// [`CanCanError::WrongAssociation`] for nested conditions.
pub fn push_condition<'query, DB>(
    builder: &mut QueryBuilder<'query, DB>,
    condition: &Condition,
    table_name: &str,
    columns: &ColumnMap,
) -> Result<(), CanCanError>
where
    DB: Database,
    i16: Encode<'query, DB> + Type<DB>,
    i32: Encode<'query, DB> + Type<DB>,
    i64: Encode<'query, DB> + Type<DB>,
    f32: Encode<'query, DB> + Type<DB>,
    f64: Encode<'query, DB> + Type<DB>,
    String: Encode<'query, DB> + Type<DB>,
    bool: Encode<'query, DB> + Type<DB>,
{
    match condition {
        Condition::All => {
            builder.push("1 = 1");
        }
        Condition::Eq { field, value } => {
            if matches!(value, DbValue::Null) {
                push_is_null(builder, table_name, columns, field, true)?;
            } else if let DbValue::List(values) = value {
                push_in(builder, table_name, columns, field, values, false)?;
            } else {
                push_comparison(builder, table_name, columns, field, value, " = ")?;
            }
        }
        Condition::Ne { field, value } => {
            if matches!(value, DbValue::Null) {
                push_is_null(builder, table_name, columns, field, false)?;
            } else if let DbValue::List(values) = value {
                push_in(builder, table_name, columns, field, values, true)?;
            } else {
                push_comparison(builder, table_name, columns, field, value, " <> ")?;
            }
        }
        Condition::In { field, values } => {
            push_in(builder, table_name, columns, field, values, false)?;
        }
        Condition::Range { field, min, max } => {
            push_qualified(builder, table_name, columns, field)?;
            builder.push(" BETWEEN ");
            push_value(builder, columns, field, min)?;
            builder.push(" AND ");
            push_value(builder, columns, field, max)?;
        }
        Condition::IsNull { field, is_null } => {
            push_is_null(builder, table_name, columns, field, *is_null)?;
        }
        Condition::And(parts) => push_joined(builder, parts, table_name, columns, " AND ", true)?,
        Condition::Or(parts) => push_joined(builder, parts, table_name, columns, " OR ", false)?,
        Condition::Not(inner) => {
            builder.push("NOT (");
            push_condition(builder, inner, table_name, columns)?;
            builder.push(")");
        }
        Condition::Nested { relation, .. } => {
            return Err(CanCanError::WrongAssociation(relation.clone()));
        }
        Condition::RawSql(sql) => {
            builder.push("(").push(sql.clone()).push(")");
        }
        _ => return Err(CanCanError::AttributeArgument),
    }
    Ok(())
}

/// Pushes the conditions selecting every record `action` may access.
///
/// This is the `sqlx` `accessible_by`: `can` conditions compose with `OR`,
/// `cannot` conditions with `AND NOT`. With no relevant `can` rule the
/// fragment matches nothing (`1 = 0`).
///
/// # Errors
///
/// Same as [`push_condition`], plus [`CanCanError::BlockInQuery`] for
/// matcher rules through [`Ability::rules_for_query`].
pub fn accessible_by<'query, DB>(
    builder: &mut QueryBuilder<'query, DB>,
    ability: &Ability,
    action: &str,
    subject_type: &str,
    table_name: &str,
    columns: &ColumnMap,
) -> Result<(), CanCanError>
where
    DB: Database,
    i16: Encode<'query, DB> + Type<DB>,
    i32: Encode<'query, DB> + Type<DB>,
    i64: Encode<'query, DB> + Type<DB>,
    f32: Encode<'query, DB> + Type<DB>,
    f64: Encode<'query, DB> + Type<DB>,
    String: Encode<'query, DB> + Type<DB>,
    bool: Encode<'query, DB> + Type<DB>,
{
    let rules = cancancan_core::Ability::rules_for_query(ability, action, subject_type)?;
    let rules = if cancancan_core::rules_compressor_enabled() {
        cancancan_core::compress(rules)
    } else {
        rules
    };

    let mut allowed: Vec<&Condition> = Vec::new();
    let mut denied: Vec<&Condition> = Vec::new();
    for rule in &rules {
        if rule.allows() {
            allowed.push(rule.condition());
        } else {
            denied.push(rule.condition());
        }
    }

    if allowed.is_empty() {
        builder.push("1 = 0");
    } else if allowed.len() == 1 && denied.is_empty() {
        push_condition(builder, allowed[0], table_name, columns)?;
    } else {
        builder.push("(");
        let mut first = true;
        for condition in &allowed {
            if !first {
                builder.push(" OR ");
            }
            first = false;
            builder.push("(");
            push_condition(builder, condition, table_name, columns)?;
            builder.push(")");
        }
        builder.push(")");
    }

    if !denied.is_empty() {
        builder.push(" AND NOT (");
        let mut first = true;
        for condition in &denied {
            if !first {
                builder.push(" OR ");
            }
            first = false;
            builder.push("(");
            push_condition(builder, condition, table_name, columns)?;
            builder.push(")");
        }
        builder.push(")");
    }
    Ok(())
}

fn push_joined<'query, DB>(
    builder: &mut QueryBuilder<'query, DB>,
    conditions: &[Condition],
    table_name: &str,
    columns: &ColumnMap,
    operator: &str,
    empty_matches_all: bool,
) -> Result<(), CanCanError>
where
    DB: Database,
    i16: Encode<'query, DB> + Type<DB>,
    i32: Encode<'query, DB> + Type<DB>,
    i64: Encode<'query, DB> + Type<DB>,
    f32: Encode<'query, DB> + Type<DB>,
    f64: Encode<'query, DB> + Type<DB>,
    String: Encode<'query, DB> + Type<DB>,
    bool: Encode<'query, DB> + Type<DB>,
{
    if conditions.is_empty() {
        builder.push(if empty_matches_all { "1 = 1" } else { "1 = 0" });
        return Ok(());
    }
    let mut first = true;
    for condition in conditions {
        if !first {
            builder.push(operator);
        }
        first = false;
        builder.push("(");
        push_condition(builder, condition, table_name, columns)?;
        builder.push(")");
    }
    Ok(())
}

fn push_comparison<'query, DB>(
    builder: &mut QueryBuilder<'query, DB>,
    table_name: &str,
    columns: &ColumnMap,
    field: &str,
    value: &DbValue,
    operator: &str,
) -> Result<(), CanCanError>
where
    DB: Database,
    i16: Encode<'query, DB> + Type<DB>,
    i32: Encode<'query, DB> + Type<DB>,
    i64: Encode<'query, DB> + Type<DB>,
    f32: Encode<'query, DB> + Type<DB>,
    f64: Encode<'query, DB> + Type<DB>,
    String: Encode<'query, DB> + Type<DB>,
    bool: Encode<'query, DB> + Type<DB>,
{
    push_qualified(builder, table_name, columns, field)?;
    builder.push(operator);
    push_value(builder, columns, field, value)
}

fn push_in<'query, DB>(
    builder: &mut QueryBuilder<'query, DB>,
    table_name: &str,
    columns: &ColumnMap,
    field: &str,
    values: &[DbValue],
    negated: bool,
) -> Result<(), CanCanError>
where
    DB: Database,
    i16: Encode<'query, DB> + Type<DB>,
    i32: Encode<'query, DB> + Type<DB>,
    i64: Encode<'query, DB> + Type<DB>,
    f32: Encode<'query, DB> + Type<DB>,
    f64: Encode<'query, DB> + Type<DB>,
    String: Encode<'query, DB> + Type<DB>,
    bool: Encode<'query, DB> + Type<DB>,
{
    if values.is_empty() {
        builder.push(if negated { "1 = 1" } else { "1 = 0" });
        return Ok(());
    }
    push_qualified(builder, table_name, columns, field)?;
    builder.push(if negated { " NOT IN (" } else { " IN (" });
    let mut first = true;
    for value in values {
        if !first {
            builder.push(", ");
        }
        first = false;
        push_value(builder, columns, field, value)?;
    }
    builder.push(")");
    Ok(())
}

fn push_is_null<'query, DB>(
    builder: &mut QueryBuilder<'query, DB>,
    table_name: &str,
    columns: &ColumnMap,
    field: &str,
    is_null: bool,
) -> Result<(), CanCanError>
where
    DB: Database,
    i16: Encode<'query, DB> + Type<DB>,
    i32: Encode<'query, DB> + Type<DB>,
    i64: Encode<'query, DB> + Type<DB>,
    f32: Encode<'query, DB> + Type<DB>,
    f64: Encode<'query, DB> + Type<DB>,
    String: Encode<'query, DB> + Type<DB>,
    bool: Encode<'query, DB> + Type<DB>,
{
    push_qualified(builder, table_name, columns, field)?;
    builder.push(if is_null { " IS NULL" } else { " IS NOT NULL" });
    Ok(())
}

fn push_qualified<DB>(
    builder: &mut QueryBuilder<'_, DB>,
    table_name: &str,
    columns: &ColumnMap,
    field: &str,
) -> Result<(), CanCanError>
where
    DB: Database,
{
    if !columns.contains_key(field) || !is_identifier(table_name) || !is_identifier(field) {
        return Err(CanCanError::AttributeArgument);
    }
    builder.push("\"").push(table_name.to_owned()).push("\".\"");
    builder.push(field.to_owned()).push("\"");
    Ok(())
}

fn push_value<'query, DB>(
    builder: &mut QueryBuilder<'query, DB>,
    columns: &ColumnMap,
    field: &str,
    value: &DbValue,
) -> Result<(), CanCanError>
where
    DB: Database,
    i16: Encode<'query, DB> + Type<DB>,
    i32: Encode<'query, DB> + Type<DB>,
    i64: Encode<'query, DB> + Type<DB>,
    f32: Encode<'query, DB> + Type<DB>,
    f64: Encode<'query, DB> + Type<DB>,
    String: Encode<'query, DB> + Type<DB>,
    bool: Encode<'query, DB> + Type<DB>,
{
    let column_type = columns
        .get(field)
        .copied()
        .ok_or(CanCanError::AttributeArgument)?;
    let value = coerce_value(value, column_type)?;
    match value {
        BoundValue::SmallInt(number) => builder.push_bind(number),
        BoundValue::Integer(number) => builder.push_bind(number),
        BoundValue::BigInt(number) => builder.push_bind(number),
        BoundValue::Float(single) => builder.push_bind(single),
        BoundValue::Double(double) => builder.push_bind(double),
        BoundValue::Text(text) => builder.push_bind(text),
        BoundValue::Bool(flag) => builder.push_bind(flag),
    };
    Ok(())
}

#[derive(Debug, PartialEq)]
enum BoundValue {
    SmallInt(i16),
    Integer(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    Text(String),
    Bool(bool),
}

fn coerce_value(value: &DbValue, column_type: ColumnType) -> Result<BoundValue, CanCanError> {
    match value {
        DbValue::Bool(flag) => match column_type {
            ColumnType::Bool => Ok(BoundValue::Bool(*flag)),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Int(number) => match column_type {
            ColumnType::SmallInt => i16::try_from(*number)
                .map(BoundValue::SmallInt)
                .map_err(|_| CanCanError::AttributeArgument),
            ColumnType::Integer => i32::try_from(*number)
                .map(BoundValue::Integer)
                .map_err(|_| CanCanError::AttributeArgument),
            ColumnType::BigInt => Ok(BoundValue::BigInt(*number)),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Float(number) => match column_type {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "adapter boundary: float columns intentionally coerce to f32"
            )]
            ColumnType::Float => Ok(BoundValue::Float(*number as f32)),
            ColumnType::Double => Ok(BoundValue::Double(*number)),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Str(text) => match column_type {
            ColumnType::Text | ColumnType::Json => Ok(BoundValue::Text(text.clone())),
            ColumnType::Uuid => uuid::Uuid::parse_str(text)
                .map_err(|_| CanCanError::AttributeArgument)
                .map(|parsed| BoundValue::Text(parsed.to_string())),
            _ => Err(CanCanError::AttributeArgument),
        },
        _ => Err(CanCanError::AttributeArgument),
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|item| item.is_ascii_alphanumeric() || item == '_')
}
