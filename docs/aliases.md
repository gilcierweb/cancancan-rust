# Action aliases

Aliases expand one action name into several. The default set mirrors the gem:

| alias    | expands to              |
|----------|-------------------------|
| `read`   | `index`, `show`         |
| `create` | `new`, `create`         |
| `update` | `edit`, `update`        |
| `destroy`| `destroy`               |

So a single `can(Some("read"), Some("Article"))` authorizes both
`can_check("index", ...)` and `can_check("show", ...)`.

Plus the two wildcards, which are *not* aliases but behave universally:

- `"manage"` matches **any** action.
- `"all"` matches **any** subject.

## Custom aliases

Register your own with `alias_action`:

```rust
let mut ability = Ability::new();
ability.alias_action(["publish", "unpublish"], "moderate")?;
ability.can(Some("moderate"), Some("Article"))?;

ability.can_check("publish", &article); // => true
```

The expansion is inspected with `aliases_for_action`:

```rust
ability.aliases_for_action("read"); // => ["index", "show"]
```

`alias_action` validates its input: aliases cannot collide with existing
*targets* (raising `CanCanError::InvalidAliasTarget`), which keeps the alias
graph acyclic. Replace the whole mapping with `clear_aliased_actions` plus
re-registration; read the current map with `aliased_actions()`.
