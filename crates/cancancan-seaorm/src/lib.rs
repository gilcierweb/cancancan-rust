//! Query filtering for `sea-query`, mirroring `accessible_by` from
//! the Ruby gem.
//!
//! Renders an [`Ability`](cancancan_core::Ability) as a `sea-query`
//! `Condition`, accepted by `sea-orm` through `QueryFilter::filter` /
//! `SelectStatement::cond_where`:
//!
//! ```rust
//! use cancancan_core::Ability;
//! use cancancan_seaorm::{accessible_by, ColumnMap, ColumnType};
//!
//! # fn condition(ability: &Ability) -> Result<sea_query::Condition, cancancan_seaorm::CanCanError> {
//! let columns: ColumnMap = [("user_id".to_owned(), ColumnType::Uuid)].into();
//! let cond = accessible_by(ability, "read", "Post", "posts", &columns)?;
//! Ok(cond)
//! # }
//! ```
//!
//! The returned condition plugs into any `sea-orm` query, including update and
//! delete: `Entity::find().filter(cond)`, `Entity::update_many().filter(cond)`.
//!
//! Value handling mirrors the gem: `can` conditions compose with `OR`,
//! `cannot` conditions with `AND NOT`, raw SQL conditions pass through, and
//! block matchers are rejected with [`CanCanError::BlockInQuery`].

pub use cancancan_core::{Ability, CanCanError, Condition, DbValue};
use sea_query::{Alias, Condition as SeaCondition, Expr, SimpleExpr, Value as SeaValue};
use std::collections::HashMap;

/// SQL column type backing a condition field for `sea-query`.
///
/// The adapter needs one declared type per field so `DbValue`s convert with
/// the right bind type, since queries reference columns dynamically by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnType {
    /// 16-bit integer (`SmallInt`, Rust `i16`).
    SmallInt,
    /// 32-bit integer (`Integer`, Rust `i32`).
    Integer,
    /// 64-bit integer (`BigInt`, Rust `i64`).
    BigInt,
    /// Single precision float (`Float`, Rust `f32`).
    Float,
    /// Double precision float (`Double`, Rust `f64`).
    Double,
    /// Text column (`Text`, Rust `String`).
    Text,
    /// Boolean column (`Boolean`, Rust `bool`).
    Bool,
    /// UUID column, validated and bound as a native UUID value.
    Uuid,
    /// JSON column, bound as text (`Json`).
    Json,
}

/// Maps field names to their column types for one subject table.
pub type ColumnMap = HashMap<String, ColumnType>;

/// Converts a [`DbValue`] to a `sea-query` bind value [`SeaValue`].
fn db_value_to_sea(value: &DbValue, column_type: ColumnType) -> Result<SeaValue, CanCanError> {
    match value {
        DbValue::Bool(flag) => match column_type {
            ColumnType::Bool => Ok(SeaValue::Bool(Some(*flag))),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Int(number) => match column_type {
            ColumnType::SmallInt => i16::try_from(*number)
                .map(|v| SeaValue::SmallInt(Some(v)))
                .map_err(|_| CanCanError::AttributeArgument),
            ColumnType::Integer => i32::try_from(*number)
                .map(|v| SeaValue::Int(Some(v)))
                .map_err(|_| CanCanError::AttributeArgument),
            ColumnType::BigInt => Ok(SeaValue::BigInt(Some(*number))),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Float(number) => match column_type {
            ColumnType::Float => Ok(SeaValue::Float(Some(as_f32(*number)))),
            ColumnType::Double => Ok(SeaValue::Double(Some(*number))),
            _ => Err(CanCanError::AttributeArgument),
        },
        DbValue::Str(text) => match column_type {
            ColumnType::Text | ColumnType::Json => {
                Ok(SeaValue::String(Some(Box::new(text.clone()))))
            }
            ColumnType::Uuid => {
                let parsed =
                    uuid::Uuid::parse_str(text).map_err(|_| CanCanError::AttributeArgument)?;
                Ok(SeaValue::Uuid(Some(Box::new(parsed))))
            }
            _ => Err(CanCanError::AttributeArgument),
        },
        _ => Err(CanCanError::AttributeArgument),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "adapter boundary: float columns intentionally coerce to f32"
)]
fn as_f32(value: f64) -> f32 {
    value as f32
}

fn column_expr(table_name: &str, columns: &ColumnMap, field: &str) -> Result<Expr, CanCanError> {
    if !is_identifier(field) {
        return Err(CanCanError::AttributeArgument);
    }
    if !columns.contains_key(field) {
        return Err(CanCanError::AttributeArgument);
    }
    Ok(Expr::col((Alias::new(table_name), Alias::new(field))))
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|item| item.is_ascii_alphanumeric() || item == '_')
}

fn column_type_of(columns: &ColumnMap, field: &str) -> ColumnType {
    *columns.get(field).unwrap_or(&ColumnType::Text)
}

/// Renders a [`Condition`] as a `sea-query` [`SimpleExpr`].
///
/// `Null` equality routes to `IS NULL`, mirroring the gem hash semantics.
/// Nested conditions need join metadata the adapter cannot infer; they
/// surface as [`CanCanError::WrongAssociation`].
///
/// # Errors
///
/// Returns [`CanCanError::AttributeArgument`] for unknown fields or value
/// mismatches, and [`CanCanError::WrongAssociation`] for nested conditions.
pub fn condition_to_sea(
    condition: &Condition,
    table_name: &str,
    columns: &ColumnMap,
) -> Result<SimpleExpr, CanCanError> {
    match condition {
        Condition::All => Ok(Expr::cust("1 = 1")),
        Condition::Eq { field, value } => match value {
            DbValue::Null => is_null_expr(table_name, columns, field, true),
            DbValue::List(values) => in_expr(table_name, columns, field, values, false),
            _ => eq_expr(table_name, columns, field, value, false),
        },
        Condition::Ne { field, value } => match value {
            DbValue::Null => is_null_expr(table_name, columns, field, false),
            DbValue::List(values) => in_expr(table_name, columns, field, values, true),
            _ => eq_expr(table_name, columns, field, value, true),
        },
        Condition::In { field, values } => in_expr(table_name, columns, field, values, false),
        Condition::Range { field, min, max } => between_expr(table_name, columns, field, min, max),
        Condition::IsNull { field, is_null } => is_null_expr(table_name, columns, field, *is_null),
        Condition::And(conditions) => {
            let parts = fold_conditions(conditions, table_name, columns)?;
            Ok(combine(parts, true))
        }
        Condition::Or(conditions) => {
            let parts = fold_conditions(conditions, table_name, columns)?;
            Ok(combine(parts, false))
        }
        Condition::Not(inner) => {
            let expr = condition_to_sea(inner, table_name, columns)?;
            Ok(expr.not())
        }
        Condition::Nested { relation, .. } => Err(CanCanError::WrongAssociation(relation.clone())),
        Condition::RawSql(sql) => Ok(SimpleExpr::Custom(sql.clone())),
        _ => Err(CanCanError::AttributeArgument),
    }
}

fn fold_conditions(
    conditions: &[Condition],
    table_name: &str,
    columns: &ColumnMap,
) -> Result<Vec<SimpleExpr>, CanCanError> {
    let mut parts: Vec<SimpleExpr> = Vec::with_capacity(conditions.len());
    for item in conditions {
        parts.push(condition_to_sea(item, table_name, columns)?);
    }
    Ok(parts)
}

fn combine(parts: Vec<SimpleExpr>, conjunctive: bool) -> SimpleExpr {
    let mut combined: Option<SimpleExpr> = None;
    for part in parts {
        combined = Some(match combined {
            Some(existing) if conjunctive => existing.and(part),
            Some(existing) => existing.or(part),
            None => part,
        });
    }
    let sql = if conjunctive { "1 = 1" } else { "1 = 0" };
    combined.unwrap_or_else(|| Expr::cust(sql))
}

fn eq_expr(
    table_name: &str,
    columns: &ColumnMap,
    field: &str,
    value: &DbValue,
    negated: bool,
) -> Result<SimpleExpr, CanCanError> {
    let column_type = column_type_of(columns, field);
    let bound = db_value_to_sea(value, column_type)?;
    let expr = column_expr(table_name, columns, field)?.eq(bound);
    Ok(if negated { expr.not() } else { expr })
}

fn in_expr(
    table_name: &str,
    columns: &ColumnMap,
    field: &str,
    values: &[DbValue],
    negated: bool,
) -> Result<SimpleExpr, CanCanError> {
    if values.is_empty() {
        let sql = if negated { "1 = 1" } else { "1 = 0" };
        return Ok(Expr::cust(sql));
    }
    let column_type = column_type_of(columns, field);
    let mut bound: Vec<SeaValue> = Vec::with_capacity(values.len());
    for value in values {
        bound.push(db_value_to_sea(value, column_type)?);
    }
    let expr = column_expr(table_name, columns, field)?.is_in(bound);
    Ok(if negated { expr.not() } else { expr })
}

fn between_expr(
    table_name: &str,
    columns: &ColumnMap,
    field: &str,
    min: &DbValue,
    max: &DbValue,
) -> Result<SimpleExpr, CanCanError> {
    let column_type = column_type_of(columns, field);
    let min_val = db_value_to_sea(min, column_type)?;
    let max_val = db_value_to_sea(max, column_type)?;
    Ok(column_expr(table_name, columns, field)?.between(min_val, max_val))
}

fn is_null_expr(
    table_name: &str,
    columns: &ColumnMap,
    field: &str,
    is_null: bool,
) -> Result<SimpleExpr, CanCanError> {
    let expr = column_expr(table_name, columns, field)?;
    Ok(if is_null {
        expr.is_null()
    } else {
        expr.is_not_null()
    })
}

/// Renders the condition selecting every record `action` may access.
///
/// This is the `sea-orm` accessor equivalent to the gem `accessible_by`: `can` conditions compose with `OR`,
/// `cannot` conditions with `AND NOT`. With no relevant `can` rule the
/// condition matches nothing (`1 = 0`).
///
/// # Errors
///
/// Same as [`condition_to_sea`], plus [`CanCanError::BlockInQuery`] for
/// matcher rules through [`Ability::rules_for_query`].
pub fn accessible_by(
    ability: &Ability,
    action: &str,
    subject_type: &str,
    table_name: &str,
    columns: &ColumnMap,
) -> Result<SeaCondition, CanCanError> {
    let rules = ability.rules_for_query(action, subject_type)?;
    let rules = if cancancan_core::rules_compressor_enabled() {
        cancancan_core::compress(rules)
    } else {
        rules
    };

    let mut allowed: Option<SimpleExpr> = None;
    let mut denied: Option<SimpleExpr> = None;
    for rule in &rules {
        let expr = condition_to_sea(rule.condition(), table_name, columns)?;
        if rule.allows() {
            allowed = Some(match allowed {
                Some(existing) => existing.or(expr),
                None => expr,
            });
        } else {
            denied = Some(match denied {
                Some(existing) => existing.or(expr),
                None => expr,
            });
        }
    }

    let allowed = allowed.unwrap_or_else(|| Expr::cust("1 = 0"));
    let combined = match denied {
        Some(expr) => allowed.and(expr.not()),
        None => allowed,
    };
    Ok(SeaCondition::all().add(combined))
}
