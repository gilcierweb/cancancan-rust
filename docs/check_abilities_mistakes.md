# Check abilities — avoid common mistakes

You know `can_check` works on an instance:

```rust
ability.can_check("destroy", &article);
```

and `cannot_check` is the opposite check. You can also ask about a **type**
instead of an instance:

```rust
// "may the user read *some* project?" — e.g. to render a link
if ability.can_check_type("create", "Project") {
    // show <a href="/projects/new">New Project</a>
}
```

## Type-level checks ignore conditions

This is the important part: when checking a **type**, any condition is
skipped and the rule matches.

```rust
ability.can_where(Some("read"), Some("Project"),
    Condition::Eq { field: "priority".into(), value: DbValue::Int(3) })?;

ability.can_check_type("read", "Project"); // => true
```

The question cannot be answered precisely — a type has no `priority` to
compare — so the port, like the gem, reads it as *"can the user read **a**
project?"* and answers `true`.

Consequences:

- A type-level check is a **hint** (should we render the button?). It is not
  a gate.
- Once you hold an instance, **always re-check** it so the conditions apply:

```rust
let project = load_project(id)?;
ability.authorize("read", &project)?; // conditions enforced here
```

This mirrors the gem's controller behavior: an `index` action has no instance
to authorize, so it authorizes the class and defers filtering to the
[query adapters](./query-adapters.md) (`accessible_by`).

The real answer to *"can the user update **all** articles?"* is a query
comparison, not a type check:

```rust
let editable: i64 = articles::table
    .accessible_by(&ability, "update")
    .count()
    .get_result(&mut conn)?;
let total: i64 = articles::table.count().get_result(&mut conn)?;
assert_eq!(editable, total);
```

## Instance checks always evaluate conditions

`can_check`/`authorize` on an instance never skip the condition tree —
equality, ranges, `Nested`, everything is enforced. Only
[`RawSql`](./hash_of_conditions.md#raw-sql-fragments) conditions are
non-matching in memory (fail-closed); detect those with
`ability.has_raw_sql(action, subject)` and enforce them through a query
adapter instead.
