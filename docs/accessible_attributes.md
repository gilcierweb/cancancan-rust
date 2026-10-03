# Accessible attributes

`cancancan-core` lets you define permissions on individual attributes of an
instance - the port of the gem's attribute-level rules (traditionally used to
feed Rails Strong Parameters).

Given users may only read a user's first and last name:

```ruby
# gem
can :read, User, [:first_name, :last_name]
```

```rust
// port
let mut ability = Ability::new();
ability.can_attributes(
    Some("read"),
    Some("User"),
    vec!["first_name".into(), "last_name".into()],
    None, // no extra condition
)?;
```

Attribute lists and conditions combine freely: the gem's `can :update, Book,
[:title], published: true` maps to `can_attributes_where`:

```rust
use cancancan_core::Condition;

ability.can_attributes_where(
    Some("update"),
    Some("Book"),
    vec!["title".into()],
    Condition::Eq { field: "published".into(), value: DbValue::Bool(true) },
)?;
```

## Checking a single attribute

```ruby
# gem
can? :read, @user, :first_name
```

```rust
// port
ability.can_check_attribute("read", &user, "first_name"); // => true
ability.can_check_attribute("read", &user, "password");   // => false
```

The mirror-image `cannot_attributes` / `cannot_check_attribute` deny specific
columns while leaving the rest allowed - attribute-level rules follow the same
"last matching rule wins" semantics as everything else.

## Listing permitted attributes

Ask for the full allowed list for an action on a subject type:

```ruby
# gem
current_ability.permitted_attributes(:read, @user)
#=> [:first_name, :last_name]
```

```rust
// port
let attrs = ability.permitted_attributes("read", "User");
// => ["first_name", "last_name"]
```

Typical uses:

- **Form builders** - render one input per permitted attribute.
- **Request validation** - intersect the inbound parameter keys with
  `permitted_attributes(action, subject_type)` before passing them to your
  model layer. In axum/actix this lives naturally in an extractor or in the
  handler before deserialization is committed (see
  [web integration](./web-integration.md)).

```rust
let allowed = ability.permitted_attributes("update", "Book");
let sanitized: serde_json::Map<_, _> = params
    .into_iter()
    .filter(|(k, _)| allowed.iter().any(|a| a == k))
    .collect();
```

## Filling forms: `attributes_for`

The gem's `attributes_for(action, subject)` returns the *values* a new record
should start with, derived from rule conditions. The port keeps it:

```rust
ability.can_where(
    Some("create"),
    Some("Project"),
    Condition::Eq { field: "active".into(), value: DbValue::Bool(true) },
)?;

let initial = ability.attributes_for("create", "Project");
// => { "active": Bool(true) } - prefill a form with these
```

Only simple equality conditions contribute values; ranges, lists, nested and
raw-SQL conditions are skipped (same as the gem).

> Attribute rules do **not** restrict queries: `accessible_by`-style adapter
> methods ignore the attribute list, exactly like the gem. Use
> `permitted_attributes` at the input layer instead.
