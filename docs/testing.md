# Testing

This is an authorization library. Testing the permissions you defined is not
important — **it is essential**.

Be careful when defining abilities, and even more careful when testing them.

## Unit-test the ability

Permission logic lives entirely in `Ability`, so full coverage does not
require request-level tests. `can_check` is callable on any ability value:

```rust
#[test]
fn user_can_only_destroy_own_projects() {
    let user = user(1);
    let own = project(user_id: 1);
    let foreign = project(user_id: 2);

    let ability = ability_for(Some(&user));

    assert!(ability.can_check("destroy", &own));
    assert!(ability.cannot_check("destroy", &foreign));
}
```

(The helper shapes — `user(...)`, `project(...)` — are whatever factory
functions your codebase already has; `RecordSubject` or your
`SubjectInstance` impls work directly.)

A table-driven style scales well when roles multiply:

```rust
#[rstest]
#[case::admin("admin", true)]
#[case::manager("manager", false)]
#[case::guest("guest", false)]
fn destroy_account(#[case] role: &str, #[case] allowed: bool) {
    let ability = ability_for(Some(&user_with_role(role)));
    assert_eq!(ability.can_check("manage", &account()), allowed);
}
```

## Query-layer tests

When you use the [query adapters](./query-adapters.md), test that the
generated query returns exactly the accessible rows — the in-memory and SQL
paths must agree:

```rust
#[test]
fn accessible_by_matches_can_check() {
    let ability = ability_for(Some(&user));
    let rows: Vec<Project> = projects::table
        .accessible_by(&ability, "read")
        .load(&mut conn)
        .unwrap();

    for row in &rows {
        assert!(ability.can_check("read", row));
    }
}
```

## Request-level tests

Keep the HTTP layer thin: log in (or inject) a user with the right
permissions and assert the status code.

```rust
// axum + tower::ServiceExt::oneshot, for example
let response = app.oneshot(
    Request::get("/articles/1")
        .header("authorization", admin_token())
        .body(Body::empty()).unwrap(),
).await.unwrap();
assert_eq!(response.status(), StatusCode::OK);
```

Complex permission matrices exercised only through HTTP lead to slow, bloated
suites. Keep request authorization tests light, and test thoroughly at the
`Ability` level as shown at the top.
