# Storing abilities in the database

The gem's idea ("store rules in DB so admins tweak them at runtime") works in
Rust as well - but the `Ability` itself remains an in-memory type. You bridge
both: load rules from the DB and register them into `Ability`.

`Permission` in the gem is sidestepped: for many Rust apps a "view entity" or
a `permissions` tables with subject_class/action/subject_id columns works fine.
Rust's `can?` helpers accept row-derived data:

```rust
pub fn ability_for(user_id: i64) -> Ability {
    let mut a = Ability::new();
    let permissions = load_permissions(user_id); // whatever you write in SQL
    for p in permissions {
        // permission: action, subject_class, subject_id
        let blocking_action = match Some(p.action.as_str()) {
            Some(action) => Some(action),
            None => None,
        };
        if let Some(subject_id) = p.subject_id {
            a.cannot_where(
                Some(&blocking_action.unwrap()),
                Some(&p.subject_class),
                Condition::Eq { field: "id".to_owned(), value: DbValue::Int(subject_id) },
            ).unwrap_or_default();
        }
    }
    a
}
```

This maps exactly to the gem's approach - loop on Permission records as
`can`/`cannot` rules.

## When to use this

A dedicated ability table is the right move only when administrators must
grant denials dynamically from the Admin console. Where rules are written in
code, precompile them into the ability client-side via a normal
`build_ability(user)` function.
