# Actix-web integration

`cancancan-actix` mirrors the gem's `ControllerAdditions` for actix-web: a
`CurrentAbility` extractor, a `check_authorization` middleware, and an
`AuthorizationError` implementing `ResponseError`.

```toml
[dependencies]
cancancan-core = "*"
cancancan-actix = "*"
actix-web = "4"
```

## Wiring the ability into requests

The extractor looks up the ability from request extensions - an `Arc<Ability>`,
a plain `Ability`, or `web::Data<Ability>`. Build it per request in an auth
middleware or extractor:

```rust
use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use actix_web::web::Data;
use cancancan_core::Ability;
use std::sync::Arc;

async fn ability_middleware(
    req: ServiceRequest,
    next: actix_web::middleware::Next<impl actix_web::body::MessageBody>,
) -> Result<actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>, Error> {
    let user = authenticate(&req);            // your auth
    let ability = ability_for(user.as_ref()); // your constructor
    req.extensions_mut().insert(Arc::new(ability));
    next.call(req).await
}
```

For simple cases (single shared ability, e.g. internal tools), registering
`web::Data` once works too, since the extractor also accepts it.

## Handlers

```rust
use actix_web::{web, Error, HttpResponse};
use cancancan_actix::CurrentAbility;

async fn show(
    ability: CurrentAbility,
    path: web::Path<i64>,
) -> Result<HttpResponse, Error> {
    let post = load_post(path.into_inner()).await?; // your repository; 404 on None
    ability.authorize("read", &post)?;              // 403 on denial
    Ok(HttpResponse::Ok().json(post))
}
```

`CurrentAbility` offers the same surface as the axum extractor:

| method | gem counterpart |
|---|---|
| `authorize(action, &instance)` | `authorize!` (marks the request, 403 on denial) |
| `authorize_type(action, "Post")` | class-level `authorize!` |
| `load_and_authorize(action, type, loader)` | `load_and_authorize_resource` (None -> 404) |
| `authorize_resource(action, &instance)` | `authorize_resource` |
| `can_check` / `can_check_type` / `cannot_check` | `can?` / `cannot?` |
| `skip_authorization_check()` | `skip_authorization_check` |

```rust
async fn edit(
    ability: CurrentAbility,
    path: web::Path<i64>,
) -> Result<HttpResponse, Error> {
    let post = ability
        .load_and_authorize("update", "Post", || load_post(path.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(post))
}
```

## Enforcing authorization app-wide

Wrap the app so handlers that forget to authorize fail closed with 500, like
the gem's `check_authorization`:

```rust
use actix_web::{middleware, App, HttpServer, web};

HttpServer::new(|| {
    App::new()
        .wrap(middleware::from_fn(check_authorization))
        .wrap(middleware::from_fn(ability_middleware))
        .route("/posts/{id}", web::get().to(show))
        .route("/health", web::get().to(health))
})
```

In actix-web, `.wrap` order is outermost-first: register
`check_authorization` **before** `ability_middleware` so the ability is in
extensions by the time the check inspects the request path.

Public handlers opt out with `skip_authorization_check()`.

## Testing

```rust
use actix_web::{test, App};

let app = test::init_service(App::new().route("/posts/{id}", web::get().to(show))).await;
let req = test::TestRequest::get().uri("/posts/1").to_request();
let res = test::call_service(&app, req).await;
assert_eq!(res.status(), actix_web::http::StatusCode::FORBIDDEN);
```

See [web integration](./web-integration.md) for the shared concepts and
[error handling](./error-handling.md) for the status-code mapping.
