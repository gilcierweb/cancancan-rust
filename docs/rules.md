# Rules: defining abilities

Rules are the core of `cancancan-core`. An `Ability` registers arbitrary rules
into a matched pipeline. Everything else depends on this definition step.

## Rule components

A rule has four parts:

- **`action`** — the action name as writing used (e.g. `"read"`).
- **`subject`** — a subject identifier like `"Post"` (a class name), an
  association name like `"post.author"`, or the wildcard `"all"`.
- **condition** — conditions data values (see [Conditions](./conditions.md)).
- **attributes** — attribute names allowed/denied under that rule (strong
  parameters semantics — see [Attributes](attributes.md)).
- *(optional)* **matcher** — callback-based rule that invokes the closure
  instead of evaluating a condition.

Rule ordering: **the last matching rule wins** (mirroring the gem's matching semantics).

## Basics

Defining a rule looks like this in Ruby:

```ruby
can :read, Post
can :read, Post, category: :news
```

And this in Rust:

```rust
let mut a = Ability::new();
a.can(Some("read"), Some("Post"))?;
a.can_where(
    Some("read"),
    Some("Post"),
    Condition::Eq {
        field: "category".to_owned(),
        value: DbValue::Str("news".to_owned()),
    },
)?;
```

It records that users can read "news" posts.

The [`attributes_for`] and [`permitted_attributes`] methods mirror the gem
helpers around rule-based columns: parameter filtering and the
condition-based form prefill.

### Wildcards

- `"manage"` covers every action; `"all"` covers every subject.
- See [aliases](./aliases.md) for the default action aliases.
