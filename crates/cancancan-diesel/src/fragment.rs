use cancancan_core::{
    Ability, CanCanError, Condition, DbValue, compress, rules_compressor_enabled,
};

/// Renders a [`Condition`] tree as a SQL `WHERE` fragment.
///
/// Identifiers are double-quoted and validated; text values are single-quoted
/// with `'` escaped as `''`. Numbers and booleans render as literals, so the
/// fragment runs on PostgreSQL, MySQL and SQLite without bind plumbing.
///
/// Nested conditions render against the relation name
/// (`"author"."name" = 'ada'`), mirroring joined association hashes: the
/// caller is expected to join that table first.
///
/// # Errors
///
/// Returns [`CanCanError::AttributeArgument`] for invalid identifiers or
/// non-finite float values.
pub fn condition_sql(condition: &Condition, table: &str) -> Result<String, CanCanError> {
    render_condition(condition, table)
}

/// Renders the `WHERE` fragment selecting every record `action` may access.
///
/// `allow` conditions join with `OR`, `deny` conditions with `AND NOT`,
/// mirroring the query composition of the Ruby gem. With no relevant `allow`
/// rule the fragment matches nothing (`1 = 0`).
///
/// Block-matcher rules cannot translate to SQL; they surface as
/// [`CanCanError::BlockInQuery`] through [`Ability::rules_for_query`].
///
/// # Errors
///
/// Returns [`CanCanError::BlockInQuery`] for matcher rules and
/// [`CanCanError::AttributeArgument`] for invalid identifiers or values.
pub fn accessible_by_sql(
    ability: &Ability,
    action: &str,
    subject_type: &str,
    table: &str,
) -> Result<String, CanCanError> {
    let rules = ability.rules_for_query(action, subject_type)?;
    let rules = if rules_compressor_enabled() {
        compress(rules)
    } else {
        rules
    };
    let mut allowed: Vec<String> = Vec::new();
    let mut denied: Vec<String> = Vec::new();
    for rule in &rules {
        let fragment = render_condition(rule.condition(), table)?;
        if rule.allows() {
            allowed.push(fragment);
        } else {
            denied.push(fragment);
        }
    }

    let allow_sql = if allowed.iter().any(|fragment| fragment == "1 = 1") {
        "1 = 1".to_owned()
    } else if allowed.is_empty() {
        "1 = 0".to_owned()
    } else {
        join_or(&allowed)
    };

    if denied.is_empty() {
        return Ok(allow_sql);
    }
    Ok(format!("({allow_sql}) AND NOT ({})", join_or(&denied)))
}

fn join_or(fragments: &[String]) -> String {
    fragments
        .iter()
        .map(|fragment| format!("({fragment})"))
        .collect::<Vec<String>>()
        .join(" OR ")
}

fn render_condition(condition: &Condition, table: &str) -> Result<String, CanCanError> {
    match condition {
        Condition::All => Ok("1 = 1".to_owned()),
        Condition::Eq { field, value } => match value {
            DbValue::Null => Ok(format!("{} IS NULL", qualified(table, field)?)),
            DbValue::List(values) => in_list(table, field, values, false),
            _ => Ok(format!(
                "{} = {}",
                qualified(table, field)?,
                value_sql(value)?
            )),
        },
        Condition::Ne { field, value } => match value {
            DbValue::Null => Ok(format!("{} IS NOT NULL", qualified(table, field)?)),
            DbValue::List(values) => in_list(table, field, values, true),
            _ => Ok(format!(
                "{} <> {}",
                qualified(table, field)?,
                value_sql(value)?
            )),
        },
        Condition::In { field, values } => in_list(table, field, values, false),
        Condition::Range { field, min, max } => Ok(format!(
            "{} BETWEEN {} AND {}",
            qualified(table, field)?,
            value_sql(min)?,
            value_sql(max)?,
        )),
        Condition::IsNull { field, is_null } => {
            let operator = if *is_null { "IS NULL" } else { "IS NOT NULL" };
            Ok(format!("{} {operator}", qualified(table, field)?))
        }
        Condition::And(conditions) => join_all(conditions, table, "AND", "1 = 1"),
        Condition::Or(conditions) => join_all(conditions, table, "OR", "1 = 0"),
        Condition::Not(inner) => Ok(format!("NOT ({})", render_condition(inner, table)?)),
        Condition::Nested {
            relation,
            condition,
        } => {
            if !is_identifier(relation) {
                return Err(CanCanError::WrongAssociation(relation.clone()));
            }
            render_condition(condition, relation)
        }
        Condition::RawSql(sql) => Ok(format!("({sql})")),
        _ => Err(CanCanError::AttributeArgument),
    }
}

fn join_all(
    conditions: &[Condition],
    table: &str,
    operator: &str,
    empty: &str,
) -> Result<String, CanCanError> {
    if conditions.is_empty() {
        return Ok(empty.to_owned());
    }
    let mut fragments: Vec<String> = Vec::with_capacity(conditions.len());
    for condition in conditions {
        fragments.push(format!("({})", render_condition(condition, table)?));
    }
    Ok(fragments.join(&format!(" {operator} ")))
}

fn in_list(
    table: &str,
    field: &str,
    values: &[DbValue],
    negated: bool,
) -> Result<String, CanCanError> {
    if values.is_empty() {
        return Ok(if negated { "1 = 1" } else { "1 = 0" }.to_owned());
    }
    let mut rendered: Vec<String> = Vec::with_capacity(values.len());
    for value in values {
        rendered.push(value_sql(value)?);
    }
    let operator = if negated { "NOT IN" } else { "IN" };
    Ok(format!(
        "{} {operator} ({})",
        qualified(table, field)?,
        rendered.join(", ")
    ))
}

fn qualified(table: &str, field: &str) -> Result<String, CanCanError> {
    if !is_identifier(table) || !is_identifier(field) {
        return Err(CanCanError::AttributeArgument);
    }
    Ok(format!("\"{table}\".\"{field}\""))
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|item| item.is_ascii_alphanumeric() || item == '_')
}

fn value_sql(value: &DbValue) -> Result<String, CanCanError> {
    match value {
        DbValue::Null => Ok("NULL".to_owned()),
        DbValue::Bool(true) => Ok("TRUE".to_owned()),
        DbValue::Bool(false) => Ok("FALSE".to_owned()),
        DbValue::Int(number) => Ok(number.to_string()),
        DbValue::Float(number) => float_sql(*number),
        DbValue::Str(text) => Ok(format!("'{}'", text.replace('\'', "''"))),
        DbValue::List(values) => {
            let mut rendered: Vec<String> = Vec::with_capacity(values.len());
            for item in values {
                rendered.push(value_sql(item)?);
            }
            Ok(format!("({})", rendered.join(", ")))
        }
        _ => Err(CanCanError::AttributeArgument),
    }
}

fn float_sql(value: f64) -> Result<String, CanCanError> {
    if value.is_finite() {
        Ok(value.to_string())
    } else {
        Err(CanCanError::AttributeArgument)
    }
}
