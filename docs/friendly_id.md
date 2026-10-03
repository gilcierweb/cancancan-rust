# FriendlyId / slug lookups

> **Status: not applicable / not ported - by design.**

The gem's FriendlyId guide patches `ActiveRecordAdapter.find` so that
`load_and_authorize_resource` can look records up by slug instead of id. That
hook exists because the gem *ships the find*: `load_resource` calls
`model_class.find(id)` internally.

`cancancan-rust` never fetches your records. Loading is your code - a
repository call, a Diesel `filter(slug.eq(&slug))`, whatever fits - and
authorization runs on the instance you loaded:

```rust
// axum-style: the slug is just your lookup key
async fn show(
    ability: CurrentAbility,
    Path(slug): Path<String>,
) -> Result<Json<Post>, AppError> {
    let post = posts::table
        .filter(posts::slug.eq(&slug))
        .first(&mut conn)
        .optional()?
        .ok_or(AppError::NotFound)?;

    ability.authorize("read", &post)?; // conditions enforced on the instance
    Ok(Json(post))
}
```

Because the port authorizes *instances and conditions* rather than the find
itself, there is no adapter method to patch - slug support needs no
integration.

If you want the safer "403 vs 404" behavior described in
[handling access denied](./handling_access_denied.md#do-not-leak-existence),
scope the lookup through the query adapter instead:

```rust
let post = posts::table
    .accessible_by(&ability, "read")
    .filter(posts::slug.eq(&slug))
    .first(&mut conn)
    .optional()?
    .ok_or(AppError::NotFound)?; // forbidden and missing are both 404
```
