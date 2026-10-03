# Integrating `rolify-rust` + `cancancan-rust` + authentication

This guide mirrors the classic Rails stack (Devise for authentication + rolify
for roles + CanCanCan for authorization) with their Rust equivalents:

| Rails     | Rust                                        | Responsibility                          |
|-----------|---------------------------------------------|-----------------------------------------|
| Devise    | `axum-login` (axum) or `password-auth` (actix) | authentication (who you are)          |
| rolify    | [`rolify-rust`](https://github.com/gilcierweb/rolify-rust)  | role assignment (what groups you belong to) |
| CanCanCan | `cancancan-rust`                            | authorization (what actions you can do) |

The three libraries are complementary, not overlapping by design:

- Authentication answers *who* is calling.
- `rolify-rust` answers what role(s) they hold.
- `cancancan-rust` decides what that role may do, per request, per record.

## Pattern 1 — Axum + axum-login (recommended for new projects)

1. Login logout via an `axum-login` auth layer.

2. Load the user's roles once per request in middleware and build the ability
   from them (plus the user id):

```rust
use axum::{extract::{Extension, Request}, middleware, routing::get, Router};
use rolify_core::RolifyUser;      // user's type implements RolifyUser
use cancancan_axum::{CurrentAbility, AuthorizationError};

async fn ability_middleware(
    mut request: Request,
    next: middleware::Next,
) -> Result<axum::response::Response, AuthorizationError> {
    // whatever your session backend stores (user id, or fetched user)
    let user_id = 1_i64; // from session
    let is_admin = true;  // via rolify: user.has_role("admin").await?

    let mut ability = cancancan_core::Ability::new();
    ability.can_where(
        Some("read"), Some("Post"),
        cancancan_core::Condition::Eq {
            field: "user_id".to_owned(),
            value: cancancan_core::DbValue::Int(user_id),
        },
    )?;
    if is_admin {
        ability.can(Some("manage"), Some("all"))?;
    }

    request.extensions_mut().insert(std::sync::Arc::new(ability));
    Ok(next.run(request).await)
}

async fn show_post(ability: CurrentAbility) -> Result<&'static str, AuthorizationError> {
    let post = ability
        .load_and_authorize::<Post, AuthorizationError>(
            "read", "Post",
            // loader closure owns the DB call:
            async { Ok(Some(Post { user_id: 1, published: true, title: String::from("demo") })) },
        )
        .await?;
    Ok("Post loaded")
}
```

3. Guard every route whose handler must authorize explicitly:

```rust
let app = Router::new()
    .route("/posts/:id", get(show_post))
    .layer(middleware::from_fn(cancancan_axum::check_authorization));
```

The middleware returns 500 on any successful handler that forgets
`authorize`, exactly the same as Rails `check_authorization`.

## Pattern 2 — Actix + password-auth

There is no canonical session crate for actix-web yet; the simplest approach is
`password-auth` (or your existing Devise replacement) plus middleware that maps
session cookies to a `User`. The authorization phase is identical for both
frameworks:

```rust
use actix_web::{web, App, middleware};
use cancancan_actix::{CurrentAbility, check_authorization};

async fn register_ability(ability: &Ability, current_user: &UserImpl) -> Result<(), cancancan_core::CanCanError> {
    if current_user.has_role(rolify_core::RoleName::from("admin")).await? {
        ability.can(Some("manage"), Some("all"))?;
    }
    /* more rules… */
    Ok(())
}

let app = App::new()
    .app_data(web::Data::new(ability)) // built per-request from the session
    .route("/posts/{id}", web::get().to(handler))
    .wrap(middleware::from_fn(check_authorization));
```

## Attaching roles from `rolify-rust` at request time

Ability construction happens exactly once per request (in middleware), not per
handler. A common recipe:

1. Fetch the current user (from `axum-login` session / actix session).
2. Query their roles with `rolify-*` (one query via your chosen adapter).
3. Materialize the ability in `ability.can_where(...)` rules, so mid-plane
   queries never need to hit the roles table again.

When checking *inside* a handler, use the extractor's `ability()` reference.
For Rails-like authorization on a controller method, call one of:

- `ability.authorize("read", &post)` — 403 if denied.
- `ability.authorize_type("update", "Post")` — class-level check.
- `ability.load_and_authorize("update", "Post", load_post(id))` — load once, then authorize.
- `ability.skip_authorization_check()` — opt out in public handlers.

## Error taxonomy (uniform across adapters)

| Failure | HTTP |
|---|---|
| `AccessDenied` | 403 |
| `can`-only matchers used for queries | [`CanCanError::BlockInQuery`](cancancan_core::CanCanError) (500) |
| Identifiers/identifiers not in a rule validate | `AttributeArgument` (500) |
| Missing record via loader | 404 (from web extractor) |
| Loader failure | 500 |

## Notes / current deferred area

- `load_and_authorize_resource` (`skip_` variants included) is currently a
  *function* accessed on the extractor. Rails-style *directives* declared on
  the controller layer are intentionally not ported (they encode Rails
  routing/hash conventions).
- The scalar alias language from the gem (`:manage`, `:all`) is preserved;
  there is no custom "my action" registration layer — use `alias_action` with
  collision validation.
