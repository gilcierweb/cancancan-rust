# Debugging abilities

When permissions don't do what you expect, checks are your friend.

## Verification first

If you are going through a slow feedback loop, start locally:

```rust
let ability = Ability::new();
let post = Post { title: String::from("hello"), user_id: 5, published: true };

ability.can_check("edit", &post);
```

`can_check` takes an instance. Rules defined with blocks/closures (`can_matching`)
only run when you pass an instance, not when you pass a type name. Model-level
checks are taken example, but they don't invoke their match expressions: rules
with conditions or strings won't run either.

## Class-level (`type`) checks

```rust
ability.can_check_type("edit", "Post");
```

Instance checks evaluate the rule's conditions and matcher. A type-level check
does not evaluate conditions; it asks whether there is *any* relevant rule
registered for a `Post` type.

Use when getting a record by id in adapters raises no
`cancancan_diesel::accessible_by(&mut connection, ...)` etc - debugging mappings
with adapter specificities is the next likely culprit.

## Denial logging

When an authorization fails, you receive a typed `CanCanError::AccessDenied`.
Turn on debug... With the default message the most common will be "You are
not authorized to read this Post" rather than vs exact matcher rules.

To use the gem equivalent of Rails' `rescue_from` block, you need to log it:

```rust
let abilities = build_ability(current_user);
if let Err(err) = abilities.authorize("destroy", &project) {
    tracing::debug!("Access denied on {}: action={} subject={}", err, err.action(), err.subject());
}
```

For controllers using [`CurrentAbility`] with the check-authorization
middleware: `AccessDenied` turns into 403 Forbidden before anything is written
to DB.
