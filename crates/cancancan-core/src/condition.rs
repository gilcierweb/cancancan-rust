use std::collections::HashMap;

/// Scalar value stored on a subject attribute or used inside a condition.
///
/// Adapter crates translate this enum into backend-bound values
/// (`diesel` bind, `sea-orm` `Value`, `sqlx` bind, `bson::Bson`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DbValue {
    /// Missing or SQL NULL value.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i64),
    /// Floating point value.
    Float(f64),
    /// Text value.
    Str(String),
    /// List value, used by `IN` conditions.
    List(Vec<DbValue>),
}

impl From<bool> for DbValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for DbValue {
    fn from(value: i32) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<i64> for DbValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<&str> for DbValue {
    fn from(value: &str) -> Self {
        Self::Str(value.to_owned())
    }
}

impl From<String> for DbValue {
    fn from(value: String) -> Self {
        Self::Str(value)
    }
}

/// Anything an ability can be checked against.
///
/// Model structs implement this trait so hash conditions can be matched
/// in memory, mirroring `ConditionsMatcher` from the Ruby gem.
pub trait SubjectInstance {
    /// Type name used to match the rule subject (e.g. `"Post"`).
    fn subject_type(&self) -> &str;

    /// Attribute value by field name, or `None` when the field is absent.
    fn attribute(&self, name: &str) -> Option<DbValue>;

    /// Associated instance for a nested (association) condition.
    fn association(&self, _name: &str) -> Option<&dyn SubjectInstance> {
        None
    }
}

/// Declarative condition tree attached to a rule.
///
/// Mirrors the hash conditions of the Ruby gem (`Eq`, `In`, `Range`,
/// nested hashes) extended with explicit boolean combinators (`And`,
/// `Or`, `Not`) so adapters can translate the tree into backend queries.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Condition {
    /// Matches every instance. This is the `can` without conditions case.
    All,
    /// Equality between a field and a value.
    Eq { field: String, value: DbValue },
    /// Inequality between a field and a value.
    Ne { field: String, value: DbValue },
    /// Membership of a field in a value list. An empty list never matches.
    In { field: String, values: Vec<DbValue> },
    /// Inclusive range check, mirroring Ruby `Range#cover?`.
    Range {
        field: String,
        min: DbValue,
        max: DbValue,
    },
    /// NULL check on a field.
    IsNull { field: String, is_null: bool },
    /// Conjunction. An empty list matches everything.
    And(Vec<Condition>),
    /// Disjunction. An empty list matches nothing.
    Or(Vec<Condition>),
    /// Negation.
    Not(Box<Condition>),
    /// Condition on an associated instance (nested hash in the gem).
    Nested {
        relation: String,
        condition: Box<Condition>,
    },
}

impl Condition {
    /// Checks this condition against an in-memory instance.
    #[must_use]
    pub fn matches(&self, instance: &dyn SubjectInstance) -> bool {
        match self {
            Self::All => true,
            Self::Eq { field, value } => instance.attribute(field).as_ref() == Some(value),
            Self::Ne { field, value } => {
                matches!(instance.attribute(field), Some(actual) if actual != *value)
            }
            Self::In { field, values } => match instance.attribute(field) {
                Some(actual) => values.contains(&actual),
                None => false,
            },
            Self::Range { field, min, max } => match instance.attribute(field) {
                Some(actual) => {
                    compare_values(&actual, min) != Some(std::cmp::Ordering::Less)
                        && compare_values(&actual, max) != Some(std::cmp::Ordering::Greater)
                }
                None => false,
            },
            Self::IsNull { field, is_null } => {
                let missing = match instance.attribute(field) {
                    None | Some(DbValue::Null) => true,
                    Some(_) => false,
                };
                missing == *is_null
            }
            Self::And(conditions) => conditions.iter().all(|item| item.matches(instance)),
            Self::Or(conditions) => conditions.iter().any(|item| item.matches(instance)),
            Self::Not(inner) => !inner.matches(instance),
            Self::Nested {
                relation,
                condition,
            } => instance
                .association(relation)
                .is_some_and(|related| condition.matches(related)),
        }
    }

    /// Collects plain equality pairs, mirroring `attributes_from_conditions`.
    ///
    /// Only [`Condition::Eq`] pairs are collected; `In`, `Range` and nested
    /// conditions are ignored exactly like the non-scalar values in the gem.
    #[must_use]
    pub fn scalar_attributes(&self) -> HashMap<String, DbValue> {
        let mut attributes = HashMap::new();
        self.collect_scalar_attributes(&mut attributes);
        attributes
    }

    fn collect_scalar_attributes(&self, attributes: &mut HashMap<String, DbValue>) {
        match self {
            Self::Eq { field, value } => {
                attributes.insert(field.clone(), value.clone());
            }
            Self::And(conditions) | Self::Or(conditions) => {
                for item in conditions {
                    item.collect_scalar_attributes(attributes);
                }
            }
            Self::All
            | Self::Ne { .. }
            | Self::In { .. }
            | Self::Range { .. }
            | Self::IsNull { .. }
            | Self::Not(_)
            | Self::Nested { .. } => {}
        }
    }
}

fn compare_values(left: &DbValue, right: &DbValue) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (DbValue::Int(left), DbValue::Int(right)) => Some(left.cmp(right)),
        (DbValue::Float(left), DbValue::Float(right)) => left.partial_cmp(right),
        (DbValue::Int(left), DbValue::Float(right)) => (*left as f64).partial_cmp(right),
        (DbValue::Float(left), DbValue::Int(right)) => left.partial_cmp(&(*right as f64)),
        (DbValue::Str(left), DbValue::Str(right)) => Some(left.cmp(right)),
        (DbValue::Bool(left), DbValue::Bool(right)) => Some(left.cmp(right)),
        _ => None,
    }
}
