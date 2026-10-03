use std::collections::HashMap;

use cancancan_core::{CanCanError, DbValue};

/// SQL column type backing a condition field.
///
/// Diesel columns are distinct types carrying their SQL type, so the adapter
/// needs one declared type per field. This enum is that declaration: a small
/// static map instead of per-model query code.
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
    /// Text (`Text`, Rust `String`).
    Text,
    /// Boolean (`Bool`, Rust `bool`).
    Bool,
    /// UUID column. Postgres binds natively (`diesel::sql_types::Uuid`);
    /// SQLite binds as validated text (UUIDs stored as text there).
    Uuid,
}

/// Maps field names to their SQL column types for one subject table.
pub type ColumnMap = HashMap<String, ColumnType>;

/// Binds `$col` to the dynamically-typed column and `$conv` to the matching
/// value-conversion function, then evaluates `$body`.
///
/// Collapses the per-SQL-type dispatch (eight near-identical arms) into one
/// definition per predicate shape. SQL types use absolute paths so the macro
/// expands in backend modules without imports. The UUID arm's column SQL
/// type and converter are parameters, because they differ per backend
/// (native on Postgres, validated text on SQLite).
macro_rules! dispatch_column {
    ($uuid_col_ty:ty, $uuid_conv:path, $column_type:expr, $dynamic:expr, $name:expr, |$col:ident, $conv:ident| $body:expr) => {
        match $column_type {
            crate::typed::ColumnType::SmallInt => {
                let $col = $dynamic.column::<diesel::sql_types::SmallInt, _>($name);
                let $conv = crate::typed::as_i16;
                $body
            }
            crate::typed::ColumnType::Integer => {
                let $col = $dynamic.column::<diesel::sql_types::Integer, _>($name);
                let $conv = crate::typed::as_i32;
                $body
            }
            crate::typed::ColumnType::BigInt => {
                let $col = $dynamic.column::<diesel::sql_types::BigInt, _>($name);
                let $conv = crate::typed::as_i64;
                $body
            }
            crate::typed::ColumnType::Float => {
                let $col = $dynamic.column::<diesel::sql_types::Float, _>($name);
                let $conv = crate::typed::as_f32;
                $body
            }
            crate::typed::ColumnType::Double => {
                let $col = $dynamic.column::<diesel::sql_types::Double, _>($name);
                let $conv = crate::typed::as_f64;
                $body
            }
            crate::typed::ColumnType::Text => {
                let $col = $dynamic.column::<diesel::sql_types::Text, _>($name);
                let $conv = crate::typed::as_string;
                $body
            }
            crate::typed::ColumnType::Bool => {
                let $col = $dynamic.column::<diesel::sql_types::Bool, _>($name);
                let $conv = crate::typed::as_bool;
                $body
            }
            crate::typed::ColumnType::Uuid => {
                let $col = $dynamic.column::<$uuid_col_ty, _>($name);
                let $conv = $uuid_conv;
                $body
            }
        }
    };
}

pub(crate) use dispatch_column;

/// Stamps `condition_predicate` and `accessible_by` for one backend.
///
/// Expression nodes (`AND`/`OR`/`NOT`) and value binds only implement their
/// traits for concrete backends (Diesel specialization internals), so every
/// function touching Diesel expressions is monomorphized per backend while
/// conversions and dispatch stay generic and shared.
macro_rules! backend_predicates {
    ($db:ty; uuid: ($uuid_col_ty:ty, $uuid_conv:path)) => {
        fn _always<QS: 'static>() -> Box<
            dyn diesel::expression::BoxableExpression<QS, $db, SqlType = diesel::sql_types::Bool>,
        > {
            use diesel::expression::IntoSql as _IntoSql;
            use diesel::expression_methods::ExpressionMethods as _ExpressionMethods;
            Box::new(1i32.into_sql::<diesel::sql_types::Integer>().eq(1))
        }

        fn _never<QS: 'static>() -> Box<
            dyn diesel::expression::BoxableExpression<QS, $db, SqlType = diesel::sql_types::Bool>,
        > {
            use diesel::expression::IntoSql as _IntoSql;
            use diesel::expression_methods::ExpressionMethods as _ExpressionMethods;
            Box::new(1i32.into_sql::<diesel::sql_types::Integer>().eq(0))
        }

        fn _or_all<QS: 'static>(
            predicates: Vec<
                Box<
                    dyn diesel::expression::BoxableExpression<
                            QS,
                            $db,
                            SqlType = diesel::sql_types::Bool,
                        >,
                >,
            >,
        ) -> Box<
            dyn diesel::expression::BoxableExpression<QS, $db, SqlType = diesel::sql_types::Bool>,
        > {
            use diesel::expression_methods::BoolExpressionMethods as _BoolExpressionMethods;
            let mut combined: Option<
                Box<
                    dyn diesel::expression::BoxableExpression<
                            QS,
                            $db,
                            SqlType = diesel::sql_types::Bool,
                        >,
                >,
            > = None;
            for predicate in predicates {
                combined = Some(match combined {
                    Some(accumulated) => Box::new(accumulated.or(predicate)),
                    None => predicate,
                });
            }
            combined.unwrap_or_else(_never::<QS>)
        }

        fn _and_all<QS: 'static>(
            predicates: Vec<
                Box<
                    dyn diesel::expression::BoxableExpression<
                            QS,
                            $db,
                            SqlType = diesel::sql_types::Bool,
                        >,
                >,
            >,
        ) -> Box<
            dyn diesel::expression::BoxableExpression<QS, $db, SqlType = diesel::sql_types::Bool>,
        > {
            use diesel::expression_methods::BoolExpressionMethods as _BoolExpressionMethods;
            let mut combined: Option<
                Box<
                    dyn diesel::expression::BoxableExpression<
                            QS,
                            $db,
                            SqlType = diesel::sql_types::Bool,
                        >,
                >,
            > = None;
            for predicate in predicates {
                combined = Some(match combined {
                    Some(accumulated) => Box::new(accumulated.and(predicate)),
                    None => predicate,
                });
            }
            combined.unwrap_or_else(_always::<QS>)
        }

        fn _eq_predicate<QS: 'static>(
            table_name: &str,
            field: &str,
            columns: &crate::typed::ColumnMap,
            value: &cancancan_core::DbValue,
            negated: bool,
        ) -> Result<
            Box<
                dyn diesel::expression::BoxableExpression<
                        QS,
                        $db,
                        SqlType = diesel::sql_types::Bool,
                    >,
            >,
            cancancan_core::CanCanError,
        > {
            use diesel::expression_methods::ExpressionMethods as _ExpressionMethods;
            let (name, column_type) = crate::typed::lookup(columns, field)?;
            let dynamic = diesel_dynamic_schema::table(table_name.to_owned());
            crate::typed::dispatch_column!(
                $uuid_col_ty,
                $uuid_conv,
                column_type,
                dynamic,
                name,
                |column, convert| {
                    let bound = convert(value)?;
                    Ok(if negated {
                        Box::new(column.ne(bound))
                    } else {
                        Box::new(column.eq(bound))
                    })
                }
            )
        }

        fn _in_predicate<QS: 'static>(
            table_name: &str,
            field: &str,
            columns: &crate::typed::ColumnMap,
            values: &[cancancan_core::DbValue],
            negated: bool,
        ) -> Result<
            Box<
                dyn diesel::expression::BoxableExpression<
                        QS,
                        $db,
                        SqlType = diesel::sql_types::Bool,
                    >,
            >,
            cancancan_core::CanCanError,
        > {
            use diesel::expression_methods::ExpressionMethods as _ExpressionMethods;
            if values.is_empty() {
                return Ok(if negated { _always() } else { _never() });
            }
            let (name, column_type) = crate::typed::lookup(columns, field)?;
            let dynamic = diesel_dynamic_schema::table(table_name.to_owned());
            crate::typed::dispatch_column!(
                $uuid_col_ty,
                $uuid_conv,
                column_type,
                dynamic,
                name,
                |column, convert| {
                    let bound = crate::typed::convert_many(values, convert)?;
                    Ok(if negated {
                        Box::new(column.ne_all(bound))
                    } else {
                        Box::new(column.eq_any(bound))
                    })
                }
            )
        }

        fn _between_predicate<QS: 'static>(
            table_name: &str,
            field: &str,
            columns: &crate::typed::ColumnMap,
            min: &cancancan_core::DbValue,
            max: &cancancan_core::DbValue,
        ) -> Result<
            Box<
                dyn diesel::expression::BoxableExpression<
                        QS,
                        $db,
                        SqlType = diesel::sql_types::Bool,
                    >,
            >,
            cancancan_core::CanCanError,
        > {
            use diesel::expression_methods::ExpressionMethods as _ExpressionMethods;
            let (name, column_type) = crate::typed::lookup(columns, field)?;
            let dynamic = diesel_dynamic_schema::table(table_name.to_owned());
            crate::typed::dispatch_column!(
                $uuid_col_ty,
                $uuid_conv,
                column_type,
                dynamic,
                name,
                |column, convert| { Ok(Box::new(column.between(convert(min)?, convert(max)?))) }
            )
        }

        fn _null_predicate<QS: 'static>(
            table_name: &str,
            field: &str,
            columns: &crate::typed::ColumnMap,
            is_null: bool,
        ) -> Result<
            Box<
                dyn diesel::expression::BoxableExpression<
                        QS,
                        $db,
                        SqlType = diesel::sql_types::Bool,
                    >,
            >,
            cancancan_core::CanCanError,
        > {
            use diesel::expression_methods::ExpressionMethods as _ExpressionMethods;
            let (name, column_type) = crate::typed::lookup(columns, field)?;
            let dynamic = diesel_dynamic_schema::table(table_name.to_owned());
            crate::typed::dispatch_column!(
                $uuid_col_ty,
                $uuid_conv,
                column_type,
                dynamic,
                name,
                |column, _convert| {
                    Ok(if is_null {
                        Box::new(column.is_null())
                    } else {
                        Box::new(column.is_not_null())
                    })
                }
            )
        }

        /// Renders a [`cancancan_core::Condition`] as a boxed Diesel predicate
        /// with typed binds.
        ///
        /// The predicate renders `table_name` and filters any query over
        /// `QS`. Values travel as bind parameters, exactly like hand-written
        /// Diesel filters.
        ///
        /// Nested conditions need join metadata the adapter cannot infer;
        /// they surface as [`cancancan_core::CanCanError::WrongAssociation`].
        ///
        /// # Errors
        ///
        /// Returns [`cancancan_core::CanCanError::AttributeArgument`] for
        /// unknown fields or value mismatches, and
        /// [`cancancan_core::CanCanError::WrongAssociation`] for nested
        /// conditions.
        pub fn condition_predicate<QS: 'static>(
            condition: &cancancan_core::Condition,
            table_name: &str,
            columns: &crate::typed::ColumnMap,
        ) -> Result<
            Box<
                dyn diesel::expression::BoxableExpression<
                        QS,
                        $db,
                        SqlType = diesel::sql_types::Bool,
                    >,
            >,
            cancancan_core::CanCanError,
        > {
            match condition {
                cancancan_core::Condition::All => Ok(_always()),
                cancancan_core::Condition::Eq { field, value } => match value {
                    cancancan_core::DbValue::Null => {
                        _null_predicate(table_name, field, columns, true)
                    }
                    cancancan_core::DbValue::List(values) => {
                        _in_predicate(table_name, field, columns, values, false)
                    }
                    _ => _eq_predicate(table_name, field, columns, value, false),
                },
                cancancan_core::Condition::Ne { field, value } => match value {
                    cancancan_core::DbValue::Null => {
                        _null_predicate(table_name, field, columns, false)
                    }
                    cancancan_core::DbValue::List(values) => {
                        _in_predicate(table_name, field, columns, values, true)
                    }
                    _ => _eq_predicate(table_name, field, columns, value, true),
                },
                cancancan_core::Condition::In { field, values } => {
                    _in_predicate(table_name, field, columns, values, false)
                }
                cancancan_core::Condition::Range { field, min, max } => {
                    _between_predicate(table_name, field, columns, min, max)
                }
                cancancan_core::Condition::IsNull { field, is_null } => {
                    _null_predicate(table_name, field, columns, *is_null)
                }
                cancancan_core::Condition::And(conditions) => {
                    let mut parts = Vec::with_capacity(conditions.len());
                    for item in conditions {
                        parts.push(condition_predicate(item, table_name, columns)?);
                    }
                    Ok(_and_all(parts))
                }
                cancancan_core::Condition::Or(conditions) => {
                    let mut parts = Vec::with_capacity(conditions.len());
                    for item in conditions {
                        parts.push(condition_predicate(item, table_name, columns)?);
                    }
                    Ok(_or_all(parts))
                }
                cancancan_core::Condition::Not(inner) => Ok(Box::new(diesel::dsl::not(
                    condition_predicate(inner, table_name, columns)?,
                ))),
                cancancan_core::Condition::Nested { relation, .. } => Err(
                    cancancan_core::CanCanError::WrongAssociation(relation.clone()),
                ),
                cancancan_core::Condition::RawSql(raw) => {
                    Ok(Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>(raw)))
                }
                _ => Err(cancancan_core::CanCanError::AttributeArgument),
            }
        }

        /// Renders the predicate selecting every record `action` may access.
        ///
        /// This is the typed `accessible_by`: `can` predicates join with
        /// `OR`, `cannot` predicates with `AND NOT`. With no relevant `can`
        /// rule the predicate matches nothing. It plugs into any Diesel
        /// query, including update and delete targets.
        ///
        /// # Errors
        ///
        /// Same as [`condition_predicate`], plus
        /// [`cancancan_core::CanCanError::BlockInQuery`] for matcher rules.
        pub fn accessible_by<QS: 'static>(
            ability: &cancancan_core::Ability,
            action: &str,
            subject_type: &str,
            table_name: &str,
            columns: &crate::typed::ColumnMap,
        ) -> Result<
            Box<
                dyn diesel::expression::BoxableExpression<
                        QS,
                        $db,
                        SqlType = diesel::sql_types::Bool,
                    >,
            >,
            cancancan_core::CanCanError,
        > {
            use diesel::expression_methods::BoolExpressionMethods as _BoolExpressionMethods;
            let mut allowed = Vec::new();
            let mut denied = Vec::new();
            let rules = ability.rules_for_query(action, subject_type)?;
            let rules = if cancancan_core::rules_compressor_enabled() {
                cancancan_core::compress(rules)
            } else {
                rules
            };
            for rule in &rules {
                let predicate = condition_predicate(rule.condition(), table_name, columns)?;
                if rule.allows() {
                    allowed.push(predicate);
                } else {
                    denied.push(predicate);
                }
            }
            let mut combined = _or_all(allowed);
            if !denied.is_empty() {
                combined = Box::new(combined.and(diesel::dsl::not(_or_all(denied))));
            }
            Ok(combined)
        }
    };
}

pub(crate) use backend_predicates;

pub(crate) fn lookup(
    columns: &ColumnMap,
    field: &str,
) -> Result<(String, ColumnType), CanCanError> {
    if !is_identifier(field) {
        return Err(CanCanError::AttributeArgument);
    }
    columns
        .get(field)
        .map(|column_type| (field.to_owned(), *column_type))
        .ok_or(CanCanError::AttributeArgument)
}

pub(crate) fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|item| item.is_ascii_alphanumeric() || item == '_')
}

pub(crate) fn as_i16(value: &DbValue) -> Result<i16, CanCanError> {
    match value {
        DbValue::Int(number) => i16::try_from(*number).map_err(|_| CanCanError::AttributeArgument),
        _ => Err(CanCanError::AttributeArgument),
    }
}

pub(crate) fn as_i32(value: &DbValue) -> Result<i32, CanCanError> {
    match value {
        DbValue::Int(number) => i32::try_from(*number).map_err(|_| CanCanError::AttributeArgument),
        _ => Err(CanCanError::AttributeArgument),
    }
}

pub(crate) fn as_i64(value: &DbValue) -> Result<i64, CanCanError> {
    match value {
        DbValue::Int(number) => Ok(*number),
        _ => Err(CanCanError::AttributeArgument),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "adapter boundary: intentional value coercion for float columns"
)]
pub(crate) fn as_f32(value: &DbValue) -> Result<f32, CanCanError> {
    match value {
        DbValue::Int(number) => Ok(*number as f32),
        DbValue::Float(number) => Ok(*number as f32),
        _ => Err(CanCanError::AttributeArgument),
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "adapter boundary: intentional value coercion for float columns"
)]
pub(crate) fn as_f64(value: &DbValue) -> Result<f64, CanCanError> {
    match value {
        DbValue::Int(number) => Ok(*number as f64),
        DbValue::Float(number) => Ok(*number),
        _ => Err(CanCanError::AttributeArgument),
    }
}

pub(crate) fn as_string(value: &DbValue) -> Result<String, CanCanError> {
    match value {
        DbValue::Str(text) => Ok(text.clone()),
        _ => Err(CanCanError::AttributeArgument),
    }
}

pub(crate) fn as_bool(value: &DbValue) -> Result<bool, CanCanError> {
    match value {
        DbValue::Bool(flag) => Ok(*flag),
        _ => Err(CanCanError::AttributeArgument),
    }
}

pub(crate) fn as_uuid_native(value: &DbValue) -> Result<uuid::Uuid, CanCanError> {
    match value {
        DbValue::Str(text) => {
            uuid::Uuid::parse_str(text).map_err(|_| CanCanError::AttributeArgument)
        }
        _ => Err(CanCanError::AttributeArgument),
    }
}

pub(crate) fn as_uuid_text(value: &DbValue) -> Result<String, CanCanError> {
    as_uuid_native(value).map(|id| id.to_string())
}

pub(crate) fn convert_many<T>(
    values: &[DbValue],
    convert: fn(&DbValue) -> Result<T, CanCanError>,
) -> Result<Vec<T>, CanCanError> {
    let mut bound = Vec::with_capacity(values.len());
    for value in values {
        bound.push(convert(value)?);
    }
    Ok(bound)
}
