use cancancan_core::{Ability, Condition, DbValue};

/// Builds the application ability.
///
/// Wire roles (e.g. from `rolify-rust`) into `allow`/`deny` declarations.
/// The last matching rule wins.
#[must_use]
pub fn build_ability(user_id: i64, is_admin: bool) -> Ability {
    let mut ability = Ability::new();

    ability
        .allow(Some("read"), Some("Post"))
        .expect("static rule is valid");

    ability
        .allow_where(
            Some("manage"),
            Some("Post"),
            Condition::Eq {
                field: "user_id".to_owned(),
                value: DbValue::Int(user_id),
            },
        )
        .expect("static rule is valid");

    if is_admin {
        ability
            .allow(Some("manage"), Some("all"))
            .expect("static rule is valid");
    }

    ability
}
