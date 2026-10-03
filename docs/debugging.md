# Debugging abilities

The gem gives you resources to better understand what rules are being
evaluated.

## Rule introspection

Introspect how rules are structured for a subject:

```rust
for rule in ability.rules_for_query("read", "Post").unwrap() {
    println!("{}", rule);
}
```

(`assert!(...)`

## Verify SQL / BSON output per adapter

You can confirm per-backend language match:

```rust
let fragment = accessible_by_sql(&ability, "read", "Post", "posts")?;
assert_eq!(fragment, "(\"posts\".\"user_id\" = $1)");
```

For Diesel: use `diesel::debug_query::<Sqlite, _>(&query).to_string()`,
which prints the full SQL with bindings.

For MongoDB `accessible_by` already returns a `bson::Document`; debug-print it
with `format!("{:?}", filter)` or `document.contains_key(...)` assertions.
