# Combine abilities

Multiple rules can target the same subject. Here the user can read projects
that are released **or** available for preview:

```rust
let mut ability = Ability::new();
ability.can_where(Some("read"), Some("Project"),
    Condition::Eq { field: "released".into(), value: DbValue::Bool(true) })?;
ability.can_where(Some("read"), Some("Project"),
    Condition::Eq { field: "preview".into(), value: DbValue::Bool(true) })?;
```

`cannot` takes the same arguments as `can` and removes permissions. It is
normally placed **after** a more generic `can`:

```rust
ability.can(Some("manage"), Some("Project"))?;
ability.cannot(Some("destroy"), Some("Project"))?;
```

Order matters.

## Precedence

**The last matching rule wins.**

```rust
// correct: the denial is registered after the broad grant
ability.can(Some("manage"), Some("Project"))?;
ability.cannot(Some("destroy"), Some("Project"))?;
```

Reversed, the `can "manage"` would override `cannot "destroy"` and the user
could delete projects again.

Adding more `can` rules does not override earlier rules — they combine with
logical OR:

```rust
ability.can_where(Some("manage"), Some("Project"),
    Condition::Eq { field: "user_id".into(), value: DbValue::Int(user.id) })?;
ability.can_where(Some("update"), Some("Project"),
    Condition::Eq { field: "locked".into(), value: DbValue::Bool(false) })?;
```

`can_check("update", &project)` is true for the owner even when the project is
locked.

## Role inheritance through ordering

The same mechanism supports inherited roles — an admin who keeps the
moderator's rules but regains `destroy`:

```rust
if user.is_moderator() {
    ability.can(Some("manage"), Some("Project"))?;
    ability.cannot(Some("destroy"), Some("Project"))?;
    ability.can(Some("manage"), Some("Comment"))?;
}

if user.is_admin() {
    // registered after the moderator block, so it wins over the `cannot`
    ability.can(Some("destroy"), Some("Project"))?;
}
```

Admin rules must be defined **after** moderator rules, so they override the
denial.

## Merging ability objects

Independent rule sets can be composed with `merge` — the port behavior for
splitting definitions across modules (see
[splitting abilities](./split_ability.md)):

```rust
let mut ability = project_abilities(&user);
ability.merge(&comment_abilities(&user));
```

Rules from `other` are appended, so merged rules take precedence where both
match — same ordering semantics as writing them inline.

Next: [define abilities with matchers](./define_abilities_with_matchers.md).
