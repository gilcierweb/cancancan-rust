# Conditions and values

Conditions are the *data* of a rule — the port of the gem's hash of
conditions. This page is the type reference; the usage guide is
[hash of conditions](./hash_of_conditions.md).

## `Condition`

```rust
pub enum Condition {
    All,                                    // `can` without conditions
    Eq { field: String, value: DbValue },
    Ne { field: String, value: DbValue },
    In { field: String, values: Vec<DbValue> },
    Range { field: String, min: DbValue, max: DbValue }, // inclusive, Ruby-style
    IsNull { field: String, is_null: bool },
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    Nested { relation: String, condition: Box<Condition> }, // association
    RawSql(String),                          // query-layer only, fail-closed
}
```

`#[non_exhaustive]` — match with a wildcard arm so future variants do not
break your code.

Key semantics:

- **Dual evaluation.** The same tree is evaluated in memory
  (`Condition::matches` against a [`SubjectInstance`](./subjects.md)) and
  translated by the [query adapters](./query-adapters.md).
- **`RawSql` is fail-closed in memory**: `matches` returns `false` for it
  (the gem *raises* in that situation; a boolean API cannot raise, so the
  rule simply does not match in memory while queries still enforce it).
  Detect with `ability.has_raw_sql(action, subject)`.
- **`And(vec![])`** matches everything; **`Or(vec![])`** matches nothing —
  the same vacuous-truth rules as the gem's compressor relies on (see
  [rules compression](./rules_compression.md)).

## `DbValue`

The untyped value carried by conditions and returned by
`SubjectInstance::attribute`:

```rust
pub enum DbValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<DbValue>), // used by `In` values
}
```

`From` impls exist for `bool`, `i32`, `i64`, `String` and `&str`, so most
conditions write naturally:

```rust
Condition::Eq { field: "published".into(), value: true.into() }
Condition::Range { field: "priority".into(), min: 1.into(), max: 3.into() }
Condition::Eq { field: "title".into(), value: "Sir".into() }
```

Equality between `DbValue`s is strict across variants — `Int(1) != Str("1")`.
Adapters bind values with the column's SQL type, so the distinction matters
in memory checks: build `DbValue`s from the same typed source as your model
attributes (which the adapter integrations do automatically).

## Matchers: the non-data escape hatch

When a rule cannot be expressed as data, attach a closure instead — see
[define abilities with matchers](./define_abilities_with_matchers.md).
Matcher rules work for in-memory checks; `rules_for_query` rejects them
(`CanCanError`) because there is no SQL to translate. The gem has the same
boundary: block rules are excluded from `accessible_by`.
