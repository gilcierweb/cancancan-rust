# Define and check abilities

`cancancan-core` is an authorization library, so the first and most
interesting thing to learn is how to define and check abilities. There are two
basic entry points on `Ability`:

```rust
ability.can(Some("read"), Some("Article"))?; // define
ability.can_check("read", &article);         // check
```

- `can` / `cannot` define who can **perform** an action on a subject.
- `can_check` (and `can_check_type`, `can_check_subject`,
  `can_check_attribute`) **check** whether a subject is authorized.

## A concrete example

Take a blog: "who can edit an article?" — "only the author."

```rust
use cancancan_core::{Ability, Condition, DbValue};

let mut ability = Ability::new();
ability.can_where(
    Some("update"),
    Some("Article"),
    Condition::Eq { field: "user_id".into(), value: DbValue::Int(user.id) },
)?;
```

Checking an instance:

```rust
ability.can_check("update", &article); // => true when article.user_id == user.id
```

Instances are checked through the `SubjectInstance` trait (implement it for
your model, or use `RecordSubject`/map-backed subjects — see
[subjects](./subjects.md)).

By default there are **no permissions**: nobody can do anything until a rule
allows it.

## Increasing permissions

`cancancan` works best when permissions build up:

```rust
// guests read published articles
ability.can_where(Some("read"), Some("Article"),
    Condition::Eq { field: "published".into(), value: DbValue::Bool(true) })?;

// logged-in users also read/update their own
if let Some(user) = &current_user {
    ability.can_where(Some("read"), Some("Article"),
        Condition::Eq { field: "user_id".into(), value: DbValue::Int(user.id) })?;
    ability.can_where(Some("update"), Some("Article"),
        Condition::Eq { field: "user_id".into(), value: DbValue::Int(user.id) })?;
}

// admins read/update everything
if current_user.as_ref().is_some_and(|u| u.admin) {
    ability.can(Some("read"), Some("Article"))?;
    ability.can(Some("update"), Some("Article"))?;
}
```

## Action aliases

The default aliases mirror the gem:

| alias    | expands to        |
|----------|-------------------|
| `read`   | `index`, `show`   |
| `create` | `new`, `create`   |
| `update` | `edit`, `update`  |
| `destroy`| `destroy`         |

So `can(Some("read"), ...)` also authorizes `can_check("show", ...)` and
`can_check("index", ...)`. Register your own with
[`alias_action`](./aliases.md).

`manage` is the super-action: it authorizes **any** action on the subject.

```rust
if is_admin {
    ability.can(Some("manage"), Some("Article"))?;
}

ability.can_check("edit", &article);    // => true for admin
ability.can_check("destroy", &article); // => true for admin
```

## Subjects

The subject is usually a type name (`"Article"`), but any string works — for
example a dashboard gate:

```rust
ability.can(Some("read"), Some("admin_dashboard"))?;
// later
ability.can_check_type("read", "admin_dashboard");
```

The special subject `"all"` matches every subject. Combined with `manage` it
grants full access:

```rust
ability.can(Some("manage"), Some("all"))?;
```

Note that this also covers `can_check_type("read", "admin_dashboard")` —
`manage` literally means any action.

> Always **check** for the specific permission you care about
> (`can_check("translate", &article)`), even if today only admins pass. Later
> you can open the action to more roles by adding one rule — no call-site
> changes needed.

## Checking abilities of other users

An `Ability` is just a value — build one for any user:

```rust
let other = ability_for(&some_user);
other.can_check("update", &article);
```

A common pattern is a constructor function (e.g. `ability_for(&User) ->
Ability`) plus a memoized accessor on your user/session type.

Next: [hash of conditions](./hash_of_conditions.md), or the
[web integration](./web-integration.md) guide to secure a real application.
