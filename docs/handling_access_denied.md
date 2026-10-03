# Handling access denied

Where the gem raises `CanCan::AccessDenied`, the port returns a typed error:
`authorize` (and siblings) returns `Result<(), CanCanError>`.

```rust
use cancancan_core::{Ability, CanCanError};

ability.authorize("update", &article)?;
```

On failure you get `CanCanError::AccessDenied`, which carries the action and
subject for inspection:

```rust
match ability.authorize("read", &article) {
    Ok(()) => { /* proceed */ }
    Err(CanCanError::AccessDenied { action, subject, message, .. }) => {
        tracing::warn!(%action, %subject, %message, "access denied");
    }
    Err(e) => return Err(e.into()),
}
```

## Custom messages

Pass a message at the call site:

```rust
ability.authorize_message("read", &article, "Unable to read this article.")?;
```

Or resolve messages centrally with a resolver - the port's answer to the
gem's I18n message lookup (see [internationalization](./internationalization.md)):

```rust
ability.set_message_resolver(|action, subject| {
    match (action, subject) {
        ("update", "Project") => Some("Not allowed to update this project.".into()),
        _ => None,
    }
});
```

`manage` and `all` keys generalize exactly like in the gem's locale files;
`ability.unauthorized_message(action, subject)` exposes the resolved string.

## Mapping to HTTP

The web crates turn the error into responses. In axum/actix, `authorize`
failures become `403 Forbidden` by default (see
[web integration](./web-integration.md)):

```rust
// axum extractor pattern
match current_ability.authorize("update", &project) {
    Ok(()) => Ok(Json(project)),
    Err(e) => Err((StatusCode::FORBIDDEN, e.to_string())),
}
```

## Do not leak existence

The gem documentation warns about an information-disclosure pattern that
applies verbatim to Rust APIs: if you load a record (404 when missing) but
redirect/403 when found-but-forbidden, an attacker can enumerate which ids
exist without being authorized.

```
GET /projects/does-not-exist        -> 404
GET /projects/exists-but-forbidden  -> 403   // leaks that the id exists
```

The safer policy is to answer **404 for both**. Scope the load through the
[query adapters](./query-adapters.md) (`accessible_by`-style queries), so a
forbidden record simply is not in the result set:

```rust
// the forbidden project is absent -> the handler returns 404 naturally
let project = projects
    .accessible_by(&ability, "read")
    .filter(pk.eq(id))
    .first(&mut conn)?; // NotFound -> 404
```

That is `load_and_authorize` done the safe way: authorize at the query layer,
not after the fetch.
