# Defining abilities — best practices

The core rule engine of `cancancan-core` takes the same design cues as the
Ruby gem, but since the DSL is now a call in a typed language, some Ruby
habits deserve adjustment.

## Prefer hash-style conditions over block matchers when data lives in the DB

```rust
// Best: defined in a dialect adapter understands.
ability.can_where(
    Some("read"),
    Some("Post"),
    Condition::Eq {
        field: "published".to_owned(),
        value: DbValue::Bool(true),
    },
)?;
```

At adapter level, `can_where` is SQL-expressible — it filters a query the
same way the rule filters a controller authorization chain. Compare with:

```rust
// Not as good: same semantics in memory only
ability.can_matching(Some("read"), Some("Post"), Arc::new(|p| {
    p.attribute("published") == Some(DbValue::Bool(true))
}))?;
```

The matcher runs only in memory; the path to hoops (database query filters,
[`cancancan-*`/adapters](../crates/)) registers an error
(`CanCanError::BlockInQuery`) when invoked. See the [conditions](conditions.md)
and [`rules`-for adapters](query-adapters.md) pages for the per-adapter call
surface.

The gem's own pointer applies: if you write a rule mixing several conditions on
the same object, translate it to one `Condition::And` (or split into two rules,
for visibility), instead of inventing your own tree structure outside the
`Condition` enum.

## Give permissions, don't take them away

The original gem recommends writing layers of increasing privileges. This
maps to a function you write per user/request, with early returns when deeper
rights don't apply:

```rust
use cancancan_core::{Ability, Condition, DbValue};

pub fn build_ability(user: &User) -> Ability {
    let mut a = Ability::new();
    a.can(Some("read"), Some("Post"))?; // for everyone, even anonymous users
    if !user.logged_in() { return a; }
    // ...
    if user.is_sysadmin() {
        a.can(Some("manage"), Some("all"))?;
    }
    a
}
```

Rule order matters: later can rules take precedence for the same action,
exactly like the gem:

```rust
let mut a = Ability::new();
a.can(Some("read"), Some("Post"))?;
a.cannot_where( // last_matching takes priority
    Some("read"),
    Some("Post"),
    Condition::Eq { field: "published".to_owned(), value: DbValue::Bool(false) },
)?;
```

Rule order *is* load-and-combine order — the gem says it and the port
honors it.
