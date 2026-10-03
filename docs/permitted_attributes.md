# Accessible attributes

You can use rule-registered attributes for both prefilling forms and
filtering incoming parameters - the Rust port of the gem's strong-parameters
family.

## Filling form fields with `attributes_for`

```rust
let defaults = ability.attributes_for("update", &subject_ref);
```

`attributes_for` merges scalar condition pairs from every allow rule, in
`HashMap` form (mirrors the gem: scalar `Eq` conditions only; `In`, `Range`,
nested struct rules, and matchers are ignored since they cannot feed a
value into a form).

## Filtering parameters with `permitted_attributes`

```rust
let attrs = ability.permitted_attributes("update", "Post");
// returns Vec<String>
```

Pass this list to whichever body-parser you use, and keep only these keys
from the user payload. `cannot` rules remove names from the set, mirroring
Rails' deny-list semantics.

## Declaring attribute rules

```rust
let mut ability = Ability::new();
ability.can_attributes(
    Some("update"),
    Some("Post"),
    vec!["title".to_owned(), "published".to_owned()],
)?;
```

The expansion is handled the same way: attribute rules are plain rules, so
adapters treat them identically to can-but-only-on-field rules. Note that
`cannot` attribute rules do not merge into the adaptation of allowed set.
