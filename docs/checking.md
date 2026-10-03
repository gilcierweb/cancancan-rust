# Checking abilities

`can`/`cannot` rules are written with the helper methods from [the core
Rust API](../crates/cancancan-core/src/ability.rs). Checking (`can?` /
`cannot?` in the gem) becomes:

- `ability.can_check(action, instance)` - instance-level check. Conditions
  and matchers are evaluated; returned as `bool`.
- `ability.can_check_type(action, "TypeName")` - type-level check. Conditions
  and matchers intentionally do not run here - only `can`/`cannot` subjects
  decide. This matches the gem's "Class" check semantics.
- `ability.cannot_check(...)` / `ability.cannot_check_type(...)` - mirrors
  `cannot?` - negated version of the above.

A living example gets passed to `RulesProcessor`:

```rust
let article = Article { user_id: 42 };
ability.can_check("show", &article);
```

The class-wide variant `Post` matches without instance checks involving
conditions/matchers:

```rust
ability.can_check_type("read", "Post");
```

And `authorize` variants enforce the rule instead: error if the rule does
not match.

```rust
ability.authorize("update", &article)  // Err(AccessDenied) when denied
```
