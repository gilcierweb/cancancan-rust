# Subjects

A **subject** is what a rule constrains and what checks run against. The gem
works with Ruby classes and live objects; the port works with *type names*
(strings) plus a trait that exposes an instance's data.

## The two faces of a subject

| context | Ruby | Rust |
|---|---|---|
| definition | `can :read, Post` | `can(Some("read"), Some("Post"))` |
| type check | `can? :create, Post` | `can_check_type("create", "Post")` |
| instance check | `can? :read, @post` | `can_check("read", &post)` |

Type names match the rule's subject string. `"all"` is the wildcard subject;
`"manage"` is the wildcard action.

## `SubjectInstance`: how the core sees your models

Instance checks need attribute access without knowing your model types. That
is the whole trait:

```rust
pub trait SubjectInstance {
    /// Type name used to match the rule subject (e.g. "Post").
    fn subject_type(&self) -> &'static str;

    /// Attribute value by field name, or `None` when absent.
    fn attribute(&self, name: &str) -> Option<DbValue>;

    /// Associated instance for a `Nested` condition (optional).
    fn association(&self, name: &str) -> Option<&dyn SubjectInstance> { None }
}
```

Implement it per model - or derive it via the framework integrations, which
blanket-provide it for Diesel `Queryable` structs / SeaORM entities:

```rust
impl SubjectInstance for Post {
    fn subject_type(&self) -> &'static str { "Post" }

    fn attribute(&self, name: &str) -> Option<DbValue> {
        match name {
            "id" => Some(DbValue::Int(self.id)),
            "published" => Some(DbValue::Bool(self.published)),
            "user_id" => Some(DbValue::Int(self.user_id)),
            _ => None,
        }
    }

    fn association(&self, name: &str) -> Option<&dyn SubjectInstance> {
        match name {
            "author" => self.author.as_deref().map(|a| a as &dyn SubjectInstance),
            _ => None,
        }
    }
}
```

`association` is what lets
[`Condition::Nested`](./hash_of_conditions.md#traversing-associations)
evaluate in memory; the query adapters resolve the same path with JOINs.

## Ad-hoc subjects

Not every subject is a model. Symbol subjects in the gem
(`can :read, :admin_dashboard`) become plain strings, checked at type level:

```rust
ability.can(Some("read"), Some("admin_dashboard"))?;
ability.can_check_type("read", "admin_dashboard"); // => true
```

Strings are compared exactly - nothing is singularized or camelized, so pick
one convention and stick to it.

## Map-backed subjects for dynamic data

For data that is not a typed struct (deserialized JSON, admin-defined
records), `MapSubject` wraps a map of attributes:

```rust
use cancancan_core::{DbValue, MapSubject};

let doc = MapSubject::new("Document", [
    ("confidential".to_owned(), DbValue::Bool(false)),
    ("owner_id".to_owned(), DbValue::Int(7)),
].into_iter().collect());
ability.can_check("read", &doc);
```

This is also the quickest way to write ability unit tests - see
[testing](./testing.md).
