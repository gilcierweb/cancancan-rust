# Cannot

Yes, sometimes you need to **remove** permissions after granting them.

Even though the default assumption is that nobody has access to anything,
there are cases where you make a broader rule first and carve out exceptions:

```rust
let mut ability = Ability::new();
ability.can(Some("manage"), Some("Project"))?;          // allow all
ability.cannot(Some("destroy"), Some("Project"))?;      // except destroy
```

This grants *any* action on `Project` **except** destroy.

The corresponding check method, [`cannot_check`](../crates/cancancan-core/src/ability.rs),
is simply the negation of [`can_check`](../crates/cancancan-core/src/ability.rs):

```rust
ability.cannot_check("destroy", &project);   // true
ability.can_check("destroy", &project);      // false
```

Matching works the same as `can` does: the last matching rule wins, so
defining `cannot` after `can` wins over a broader earlier `can`. If you
reverse the order, the exception wins - which is dangerous.

The gem's own distinction holds:  `cannot` is the normalised complement of
`can` - same rules, same ordering, same adapters.
