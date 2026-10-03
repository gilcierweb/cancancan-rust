# Defining abilities — conditions

In [Define and check abilities](./define_check_abilities.md) we wrote:

```rust
ability.can_where(
    Some("update"),
    Some("Article"),
    Condition::Eq { field: "user_id".into(), value: DbValue::Int(user.id) },
)?;
```

to say an article can only be updated by its author. The third argument of
`can_where` is the **condition** — the port of the gem's *hash of
conditions*. It restricts which records the permission applies to.

Conditions are data, not callbacks: they are evaluated in memory by
`can_check` **and** translated into SQL/NoSQL by the
[query adapters](./query-adapters.md). One definition, two enforcement
points.

## Equality and conjunction

In the gem you write a hash; here you compose `Condition` values. Multiple
keys in a Ruby hash become `Condition::And`:

```ruby
# gem
can :read, Project, active: true, user_id: user.id
```

```rust
// port
ability.can_where(
    Some("read"),
    Some("Project"),
    Condition::And(vec![
        Condition::Eq { field: "active".into(), value: DbValue::Bool(true) },
        Condition::Eq { field: "user_id".into(), value: DbValue::Int(user.id) },
    ]),
)?;
```

A field name is always a database column of the model, or an association
path (see *Nested* below).

## Lists and ranges

Match any of several values with `In`, or an inclusive range with `Range`
(mirroring Ruby's `Range#cover?`):

```ruby
can :read, Project, priority: 1..3
can :read, Project, state: %w[open reopened]
```

```rust
ability.can_where(Some("read"), Some("Project"),
    Condition::Range { field: "priority".into(),
                       min: DbValue::Int(1), max: DbValue::Int(3) })?;

ability.can_where(Some("read"), Some("Project"),
    Condition::In { field: "state".into(),
                    values: vec![DbValue::Str("open".into()),
                                 DbValue::Str("reopened".into())] })?;
```

## NULL checks and negation

The gem uses `nil` for negative/NULL matches; the port makes it explicit
with `IsNull` (and `Ne` for inequality):

```ruby
can :read, Project, assignee_id: nil   # unassigned projects
```

```rust
use cancancan_core::Condition::{IsNull, Ne, Not};

ability.can_where(Some("read"), Some("Project"),
    IsNull { field: "assignee_id".into(), is_null: true })?;

ability.can_where(Some("read"), Some("Project"),
    Ne { field: "state".into(), value: DbValue::Str("archived".into()) })?;

ability.can_where(Some("read"), Some("Project"),
    Not(Box::new(IsNull { field: "owner_id".into(), is_null: true })))?;
```

## Traversing associations

Nested hashes in the gem become `Condition::Nested`, keyed by the
association name. Associations can be traversed to any depth:

```ruby
# gem: Part belongs_to :service; Service belongs_to :account;
#      Account has_one :user
can :manage, Part, service: { account: { user: user } }
```

```rust
use cancancan_core::Condition::{Eq, Nested};

ability.can_where(
    Some("manage"),
    Some("Part"),
    Nested {
        relation: "service".into(),
        condition: Box::new(Nested {
            relation: "account".into(),
            condition: Box::new(Eq {
                field: "user_id".into(),
                value: DbValue::Int(user.id),
            }),
        }),
    },
)?;
```

The query adapters turn each `Nested` level into a JOIN/subquery; in-memory
`can_check` resolves the association through `SubjectInstance`.

## Raw SQL fragments

For anything the structured variants cannot express, `Condition::RawSql`
mirrors the gem's string conditions:

```rust
ability.can_where(Some("read"), Some("Project"),
    Condition::RawSql("projects.published_at IS NOT NULL".into()))?;
```

> Raw SQL is only enforceable at the **query** layer. In-memory
> `can_check` treats a `RawSql` rule as *non-matching* (fail-closed): the
> gem raises in that situation, and a boolean API cannot raise. Prefer
> structured conditions whenever possible; check
> `ability.has_raw_sql(action, subject)` to detect rules that need a
> query adapter.

Next: [combine abilities](./combine_abilities.md).
