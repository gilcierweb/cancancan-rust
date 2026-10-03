# Web integration

The web crates — [`cancancan-axum`](../crates/cancancan-axum) and
[`cancancan-actix`](../crates/cancancan-actix) — mirror the gem's
`ControllerAdditions`. Each supplies:

- an **extractor** `CurrentAbility` that pulls the current request's `Ability`
  from request extensions (axum `Extension` / actix `web::Data`), and offers:
  - `authorize(action, instance)` / `authorize_type(action, type_name)`
    — mirrors `authorize!`. Fails with `403` on denial and marks the request.
  - `load_and_authorize(action, subject_type, loader)` — mirror of
    `load_and_authorize_resource`: runs a loader returning `Option<Instance>`,
    maps `None` to `404`, loader errors to `500`, denies to `403`.
    Locks the same internal flag.
  - `authorize_resource(action, instance)` — only the permission check
    (`mirror authorize_resource!`).
  - `skip_authorization_check()` — call this freely when you intend not to
    authorize (HTML metaboxes etc.).
  - `can_check(action, subject)` / `cannot_check` — mirrors `can?` / `cannot?`.
  - **authorize for routes** — `middleware::from_fn(check_authorization)` wraps
    the whole router and turns a missing-ability *and* missing-call case into
    a `500` (mirrors the gem's `check_authorization`/authorize-header flow);
    success-satisfied handlers without a call also surface as 500.
  - `AuthorizationError` — wraps `CanCanError`; maps to the right HTTP codes.

## Axum example

```rust
use axum::{Router, routing::get, middleware};
use cancancan_axum::{CurrentAbility, check_authorization};

async fn show(
    ability: CurrentAbility,
) -> Result<&'static str, cancancan_axum::AuthorizationError> {
    ability.authorize("read", &Post { user_id: 1 })?;
    Ok("show")
}

async fn health(_ability: CurrentAbility) -> &'static str {
    _ability.skip_authorization_check();
    "ok"
}

let app = Router::new()
    .route("/posts/{id}", get(show))
    .route("/healthy", get(health))
    .layer(middleware::from_fn(check_authorization))
    .layer(axum::extract::Extension(std::sync::Arc::new(ability_for(1, false))));
```

## Actix-web example

```rust
use actix_web::{App, web, middleware};
use cancancan_actix::{CurrentAbility, check_authorization};

async fn show(ability: CurrentAbility) -> Result<&'static str, actix_web::Error> {
    ability.authorize("read", &Post { user_id: 1 })?;
    Ok("show")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    actix_web::HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(ability_for(1, false)))
            .route("/posts/{id}", web::get().to(show))
            .wrap(middleware::from_fn(check_authorization))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

Note on public routes: call `ability.skip_authorization_check()` when you
handle calls that do authorization intentionally (for example for tiles) so
the middleware knows.
