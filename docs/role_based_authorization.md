# Role-based authorization

`cancancan-core` is decoupled from how you model roles — it only sees the
rules you register. This page shows the common Rust setups, adapted from the
gem's role guide. For roles persisted in a database with a dedicated crate,
see [INTEGRATION.md](./INTEGRATION.md) (`rolify-rust`).

## Roles in a constant

The tight coupling between role list and abilities argues for keeping roles
next to the user type:

```rust
pub const ROLES: [&str; 4] = ["admin", "moderator", "author", "banned"];
```

## One role per user

A `role: String` column on `users` and a check in the ability constructor:

```rust
pub fn ability_for(user: Option<&User>) -> Ability {
    let mut a = Ability::new();
    if user.is_some_and(|u| u.role == "admin") {
        a.can(Some("manage"), Some("all")).ok();
    }
    a
}
```

## Many roles per user

Two idiomatic options:

**A join table / array column** (`Vec<String>` on the user, a `roles` table,
or `rolify-rust`), checked with `iter().any`:

```rust
if user.roles.iter().any(|r| r == "admin") {
    ability.can(Some("manage"), Some("all"))?;
}
```

**A bitmask** in one integer column, mirroring the gem's `roles_mask` recipe:

```rust
pub fn roles_from_mask(mask: i64) -> Vec<&'static str> {
    ROLES.iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, r)| *r)
        .collect()
}

pub fn roles_to_mask(roles: &[&str]) -> i64 {
    roles.iter().fold(0, |acc, r| {
        acc | ROLES.iter().position(|x| x == r).map_or(0, |i| 1 << i)
    })
}
```

## Role inheritance

Put the ordering logic in one place. The gem's `role?(base)` comparison maps
directly to an ordered slice:

```rust
const ROLES_BY_POWER: [&str; 3] = ["moderator", "admin", "superadmin"];

fn role_at_least(user_role: &str, base: &str) -> bool {
    let rank = |r| ROLES_BY_POWER.iter().position(|&x| x == r);
    matches!((rank(user_role), rank(base)), (Some(u), Some(b)) if u >= b)
}

pub fn ability_for(user: &User) -> Ability {
    let mut a = Ability::new();
    if role_at_least(&user.role, "moderator") {
        a.can(Some("manage"), Some("Post")).ok();
    }
    if role_at_least(&user.role, "admin") {
        a.can(Some("manage"), Some("ForumThread")).ok();
    }
    if role_at_least(&user.role, "superadmin") {
        a.can(Some("manage"), Some("Forum")).ok();
    }
    a
}
```

A superadmin manages all three; a moderator only `Post`.

## Inheritance inside the ability builder

Alternatively, keep the chain in the builder itself — one function per role,
higher roles call lower ones:

```rust
pub fn ability_for(user: Option<&User>) -> Ability {
    let mut a = Ability::new();
    match user.map(|u| u.role.as_str()) {
        Some("admin") => { manager(&mut a); admin(&mut a); }
        Some("manager") => manager(&mut a),
        _ => { a.can(Some("read"), Some("all")).ok(); } // guest
    }
    a
}

fn manager(a: &mut Ability) {
    a.can(Some("manage"), Some("Employee")).ok();
}

fn admin(a: &mut Ability) {
    a.can(Some("manage"), Some("Bill")).ok();
}
```

This composes naturally with [`merge`](./combine_abilities.md#merging-ability-objects)
when abilities live in different modules — see
[splitting abilities](./split_ability.md).
