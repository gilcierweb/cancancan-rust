# Axum integration

`cancancan-axum` mirrors the gem's `ControllerAdditions` for axum: a
`CurrentAbility` extractor, a `check_authorization` middleware, and an
`AuthorizationError` that maps to HTTP status codes.

```toml
[dependencies]
cancancan-core = "*"
cancancan-axum = "*"
axum = "0.8"
```

## Wiring the ability into requests

The extractor reads an `Arc<Ability>` from request **extensions**. Build the
ability per request in your auth middleware (after the user is known) and
insert it:

```rust
use axum::{extract::Request, middleware::Next, response::Response};
use cancancan_core::Ability;
use std::sync::Arc;

async fn ability_middleware(mut req: Request, next: Next) -> Response {
    let user = authenticate(&req);             // your auth
    let ability = ability_for(user.as_ref());  // your constructor
    req.extensions_mut().insert(Arc::new(ability));
    next.run(req).await
}
```

## Handlers

```rust
use axum::{Json, extract::Path};
use cancancan_axum::{AuthorizationError, CurrentAbility};

async fn show(
    ability: CurrentAbility,
    Path(id): Path<i64>,
) -> Result<Json<Post>, AuthorizationError> {
    let post = load_post(id).await?;      // your repository; 404 on None
    ability.authorize("read", &post)?;    // 403 on denial
    Ok(Json(post))
}

async fn index(ability: CurrentAbility) -> Result<Json<Vec<Post>>, AuthorizationError> {
    ability.authorize_type("read", "Post")?;  // class-level gate
    let posts = readable_posts(&ability).await?; // query adapter, see below
    Ok(Json(posts))
}
```

`CurrentAbility` offers:

| method | gem counterpart | notes |
|---|---|---|
| `authorize(action, &instance)` | `authorize!` | marks the request, 403 on denial |
| `authorize_type(action, "Post")` | `authorize! :read, Post` | class-level |
| `load_and_authorize(action, type, loader)` | `load_and_authorize_resource` | `None` -> 404, loader err -> 500, denial -> 403 |
| `authorize_resource(action, &instance)` | `authorize_resource` | check only |
| `can_check` / `can_check_type` / `cannot_check` | `can?` / `cannot?` | no flag marking |
| `skip_authorization_check()` | `skip_authorization_check` | for public handlers |

## `load_and_authorize`

```rust
async fn edit(
    ability: CurrentAbility,
    Path(id): Path<i64>,
) -> Result<Json<Post>, AuthorizationError> {
    let post = ability
        .load_and_authorize("update", "Post", || load_post(id))
        .await?;
    Ok(Json(post))
}
```

One call gives you gem semantics: missing record is 404, denied is 403, and
the request is marked authorized.

## Enforcing authorization app-wide

Wrap the router so a handler that *forgets* to authorize fails closed (500,
like the gem's `check_authorization`):

```rust
use axum::{Router, middleware, routing::get};
use cancancan_axum::check_authorization;

let app = Router::new()
    .route("/posts/{id}", get(show).put(update))
    .route("/health", get(health))
    .layer(middleware::from_fn(check_authorization))
    .layer(middleware::from_fn(ability_middleware));
```

Public handlers opt out explicitly:

```rust
async fn health(ability: CurrentAbility) -> &'static str {
    ability.skip_authorization_check();
    "ok"
}
```

Layer order matters: `check_authorization` must sit **inside**
`ability_middleware` (last `.layer` runs first in axum), so every request has
an ability before the check runs - missing ability also surfaces as 500.

## Testing

```rust
use tower::ServiceExt; // oneshot

let app = app(); // your router
let res = app.oneshot(
    Request::get("/posts/1").body(Body::empty()).unwrap(),
).await.unwrap();
assert_eq!(res.status(), StatusCode::FORBIDDEN);
```

See [web integration](./web-integration.md) for the shared concepts and
[error handling](./error-handling.md) for the full status-code mapping.
