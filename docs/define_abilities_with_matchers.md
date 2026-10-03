# Define abilities with matchers

Matcher rules are the direct port of the gem's "block" abilities. They are
relevant only for instance-level checks — database adapters reject them
(`BlockInQuery`).

## `can_matching` and `cannot_matching`

A matcher receives the subject by reference (`&dyn SubjectInstance`) and
returns `bool`:

```rust
use cancancan_core::{Ability, DbValue, SubjectInstance};
use std::sync::Arc;

let mut a = Ability::new();
a.can_matching(
    Some("update"),
    Some("Post"),
    Arc::new(|post: &dyn SubjectInstance| {
        !post.attribute("published").unwrap_or(&DbValue::Bool(true)).to_bool().unwrap()
    }),
)?;
```

The closure is stored as a shared thread-safe (`Send + Sync`) handler: rules
can be shared across middleware / threads with this guarantee.

## Matcher vs hash conditions

Conditions are translated into the adapter, while matchers stay in the Ruby equivalent of memory.

- [`rules_for_query`](../crates/cancancan-core/src/ability.rs) automatically
  strips matcher rules when constructing filters.
- Adapters return `BlockInQuery` when reuse conflicts are detected (i.e.,
  middleware scenarios, not before).

## Matching matcher rules without querying

Opt-out with plain has_matcher detection:

```rust
let has = ability.has_matcher("update", "Post");
```
Catch-and-drop matching can work up to and including testing — an alternative to
the gem's "assume falsely" approach.

## When to write matchers

- Full logic of the form ("any post, related comment, likes") — integrity
  rules that can't be expressed in SQL at compile time.
- Frozen state (checks that depend on receiving data by pointer).
