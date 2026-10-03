# Split the ability definitions

When an application grows, one giant ability constructor stops scaling. The
idiomatic Rust split mirrors the gem's "per-model abilities folder": one
module per domain, each exposing a registration function.

```rust
// src/abilities/mod.rs
pub mod user;
pub mod book;
pub mod comment;

pub fn ability_for(user: &User) -> Ability {
    let mut ability = Ability::new();
    user::register(&mut ability, user);
    book::register(&mut ability, user);
    comment::register(&mut ability, user);
    ability
}
```

```rust
// src/abilities/book.rs
use cancancan_core::{Ability, Condition, DbValue};
use crate::models::User;

pub fn register(ability: &mut Ability, user: &User) {
    ability.can_where(Some("read"), Some("Book"),
        Condition::Eq { field: "published".into(), value: DbValue::Bool(true) }).ok();
    ability.can_where(Some("edit"), Some("Book"),
        Condition::Eq { field: "user_id".into(), value: DbValue::Int(user.id) }).ok();
}
```

Registering into a shared `Ability` keeps one important property: rule
**ordering across modules is preserved** (registration order = definition
order), so the precedence rules from
[combining abilities](./combine_abilities.md) behave exactly as if everything
were written in one function.

## Per-domain ability values

Alternatively, build one ability per domain and use only what a handler needs
— the gem's per-controller `current_ability` override:

```rust
// src/abilities/book.rs
pub fn book_ability(user: &User) -> Ability {
    let mut a = Ability::new();
    a.can_where(Some("read"), Some("Book"), /* ... */).ok();
    a
}

// in an axum handler
async fn show(user: AuthUser, Path(id): Path<i64>) -> Result<Json<Book>, AppError> {
    let ability = book_ability(&user.0); // only book rules for this handler
    let book = load_book(id)?;
    ability.authorize("read", &book)?;
    Ok(Json(book))
}
```

The handler constructs only the ruleset it needs — the saving the gem
advertises for controllers applies equally here.

## Merging

Compose independent values with `merge` (see
[combining abilities](./combine_abilities.md#merging-ability-objects)):

```rust
let mut ability = read_ability(&user);
ability.merge(&write_ability(&user));
```

Merged rules are appended, so later-merged abilities win on conflicts.
