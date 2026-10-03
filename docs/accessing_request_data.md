# Accessing request data

What if permissions depend on something outside the user object — say,
forbidding certain IP addresses from creating comments? In the gem you
override `current_ability` in `ApplicationController` to pass
`request.remote_ip` in. The port has no controller mixin: **you** own the
ability constructor, so just pass the request context in.

```rust
pub struct RequestContext {
    pub user: Option<User>,
    pub remote_ip: Option<IpAddr>,
}

pub fn ability_for(ctx: &RequestContext) -> Ability {
    let mut a = Ability::new();

    if !denylist::contains(ctx.remote_ip) {
        a.can(Some("create"), Some("Comment")).ok();
    }

    if let Some(user) = &ctx.user {
        // ... user-based rules
    }
    a
}
```

## In axum

Build the context in an extractor (see
[web integration](./web-integration.md) for `CurrentAbility`):

```rust
async fn handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    user: Option<AuthUser>, // your auth extractor
) -> Result<Json<Comment>, AppError> {
    let ability = ability_for(&RequestContext {
        user: user.map(|u| u.0),
        remote_ip: Some(addr.ip()),
    });
    ability.authorize("create", &comment)?;
    // ...
}
```

## In actix-web

Same idea — the `CurrentAbility` extractor receives the
`HttpRequest`-derived data, so peer IP, headers or session values are all
available before rules are registered.

## What belongs in the context

Anything the rules may legitimately depend on:

- **IP deny/allow lists** (as above)
- **session / cookie state** (e.g. "accepted terms" flag)
- **tenant header** (`X-Tenant-Id` scoping every condition)
- **feature flags** for gradual rollouts

Resist moving *authorization data* into the context — roles and ownership
belong in the database. The context is for request-scoped facts only; rules
stay pure and testable:

```rust
#[test]
fn denylisted_ip_cannot_create_comments() {
    let ctx = RequestContext { user: None, remote_ip: Some(BAD_IP) };
    assert!(ability_for(&ctx).cannot_check_type("create", "Comment"));
}
```
