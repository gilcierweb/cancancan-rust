# Rules compression

Databases are good at optimizing queries, but a long rule list can still
produce needlessly complex `WHERE`/`JOIN` clauses. Like the gem, the port
compresses rules **at the adapter layer, automatically**, before translating
them to a query. (`can_check` always evaluates the full rule list - in memory
the cost is negligible and precedence semantics are simpler unmodified.)

The compressor lives in `cancancan_core::compress` and is enabled by default.
Toggle it globally:

```rust
cancancan_core::set_rules_compressor_enabled(false);
```

mirroring the gem's `CanCan.rules_compressor_enabled = false`.

A rule without conditions is a *catch-all*.

## A catch-all eliminates all previous rules of the same kind

```rust
ability.can_where(Some("read"), Some("Book"), eq("author_id", user.id))?;
ability.cannot_where(Some("read"), Some("Book"), eq("private", true))?;
ability.can(Some("read"), Some("Book"))?;               // catch-all
ability.can_where(Some("read"), Some("Book"), eq("id", 1))?;
ability.cannot_where(Some("read"), Some("Book"), eq("private", true))?;
```

compresses to:

```rust
ability.can(Some("read"), Some("Book"))?;
ability.cannot_where(Some("read"), Some("Book"), eq("private", true))?;
```

Everything before the last unconditional `can` is unreachable - the catch-all
already grants access, and later rules re-state the only exception.

## A leading catch-all `cannot` is removable

Since permissions default to denied, an unconditional denial at the front
adds nothing:

```rust
ability.cannot(Some("read"), Some("Book"))?;             // redundant
ability.can_where(Some("read"), Some("Book"), eq("author_id", user.id))?;
```

becomes just:

```rust
ability.can_where(Some("read"), Some("Book"), eq("author_id", user.id))?;
```

## A rule list of only `cannot`s is equivalent to no rules

```rust
ability.cannot_where(Some("read"), Some("Book"), eq("private", true))?;
```

compresses to *nothing*: everything is denied by default anyway, and the
adapter emits a zero-rows query (see [model adapters](./model_adapter.md)).

## Why it matters

These optimizations let you follow
[*give permissions, don't take them away*](./define_abilities_best_practices.md)
without paying for the intermediate rules at query time. Broad grants plus a
few `cannot` exceptions stay cheap in SQL.
