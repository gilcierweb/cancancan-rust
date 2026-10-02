mod common;

use cancancan_core::{Ability, CanCanError, Condition, DbValue};
use common::{MapSubject, Post, User};
use std::collections::HashMap;
use std::rc::Rc;

fn owner_ability() -> Ability {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Eq {
                field: "user_id".to_owned(),
                value: DbValue::Int(1),
            },
        )
        .unwrap();
    ability
}

#[test]
fn allows_matching_rule_and_denies_others() {
    let ability = owner_ability();
    assert!(ability.can_check("read", &Post::owned(1, 1)));
    assert!(!ability.can_check("read", &Post::owned(2, 2)));
}

#[test]
fn cannot_check_matches_cannot_query() {
    let ability = owner_ability();
    assert!(ability.cannot_check("read", &Post::owned(2, 2)));
    assert!(!ability.cannot_check("read", &Post::owned(1, 1)));
}

#[test]
fn last_matching_rule_wins() {
    let mut ability = Ability::new();
    ability.can(Some("read"), Some("Post")).unwrap();
    ability
        .cannot_where(
            Some("read"),
            Some("Post"),
            Condition::Eq {
                field: "published".to_owned(),
                value: DbValue::Bool(false),
            },
        )
        .unwrap();

    let draft = Post {
        published: false,
        ..Post::owned(1, 1)
    };
    let released = Post {
        published: true,
        ..Post::owned(2, 1)
    };
    assert!(!ability.can_check("read", &draft));
    assert!(ability.can_check("read", &released));
}

#[test]
fn manage_matches_every_action_and_all_matches_every_subject() {
    let mut ability = Ability::new();
    ability.can(Some("manage"), Some("all")).unwrap();

    assert!(ability.can_check("destroy", &Post::owned(1, 9)));
    assert!(ability.can_check_type("anything", "Anything"));
}

#[test]
fn default_aliases_expand_read_create_and_update() {
    let mut ability = Ability::new();
    ability.can(Some("read"), Some("Post")).unwrap();
    ability.can(Some("create"), Some("Post")).unwrap();
    ability.can(Some("update"), Some("Post")).unwrap();

    let post = Post::owned(1, 1);
    assert!(ability.can_check("index", &post));
    assert!(ability.can_check("show", &post));
    assert!(ability.can_check_type("new", "Post"));
    assert!(ability.can_check("edit", &post));
    assert!(!ability.can_check("destroy", &post));
}

#[test]
fn custom_alias_applies_to_checks() {
    let mut ability = Ability::new();
    ability.alias_action(["show"], "preview");
    ability.can(Some("preview"), Some("Post")).unwrap();

    assert!(ability.can_check_type("show", "Post"));
    assert!(ability.can_check_type("preview", "Post"));
}

#[test]
fn class_level_check_returns_rule_behavior() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Eq {
                field: "user_id".to_owned(),
                value: DbValue::Int(1),
            },
        )
        .unwrap();
    assert!(ability.can_check_type("read", "Post"));

    let mut denied = Ability::new();
    denied
        .cannot_where(
            Some("read"),
            Some("Post"),
            Condition::Eq {
                field: "user_id".to_owned(),
                value: DbValue::Int(1),
            },
        )
        .unwrap();
    assert!(!denied.can_check_type("read", "Post"));
}

#[test]
fn block_matcher_decides_at_check_time() {
    let mut ability = Ability::new();
    ability
        .can_matching(
            Some("read"),
            Some("Post"),
            Rc::new(|instance| instance.attribute("published") == Some(DbValue::Bool(true))),
        )
        .unwrap();

    let draft = Post {
        published: false,
        ..Post::owned(1, 1)
    };
    let released = Post {
        published: true,
        ..Post::owned(2, 1)
    };
    assert!(ability.can_check("read", &released));
    assert!(!ability.can_check("read", &draft));
    assert!(ability.can_check_type("read", "Post"));
}

#[test]
fn action_without_subject_is_rejected() {
    let mut ability = Ability::new();
    let result = ability.can(Some("read"), None);
    assert_eq!(result.unwrap_err(), CanCanError::ActionWithoutSubject);
}

#[test]
fn authorize_ok_on_success_and_denied_on_failure() {
    let ability = owner_ability();
    assert!(ability.authorize("read", &Post::owned(1, 1)).is_ok());

    let error = ability.authorize("read", &Post::owned(2, 2)).unwrap_err();
    assert_eq!(
        error,
        CanCanError::AccessDenied {
            action: "read".to_owned(),
            subject: "Post".to_owned(),
            message: Some("You are not authorized to read this Post.".to_owned()),
        }
    );
}

#[test]
fn custom_message_resolver_overrides_default() {
    let mut ability = owner_ability();
    ability.set_message_resolver(Rc::new(|_, _| Some("Nao autorizado.".to_owned())));
    let error = ability.authorize("read", &Post::owned(2, 2)).unwrap_err();
    assert_eq!(
        error,
        CanCanError::AccessDenied {
            action: "read".to_owned(),
            subject: "Post".to_owned(),
            message: Some("Nao autorizado.".to_owned()),
        }
    );
}

#[test]
fn merge_combines_rules_with_last_write_wins() {
    let mut admin = Ability::new();
    admin.can(Some("manage"), Some("all")).unwrap();

    let mut combined = owner_ability();
    combined.merge(&admin);
    assert!(combined.can_check("destroy", &Post::owned(9, 9)));
}

#[test]
fn attributes_for_collects_scalar_equalities() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("create"),
            Some("Post"),
            Condition::And(vec![
                Condition::Eq {
                    field: "user_id".to_owned(),
                    value: DbValue::Int(1),
                },
                Condition::In {
                    field: "id".to_owned(),
                    values: vec![DbValue::Int(1), DbValue::Int(2)],
                },
            ]),
        )
        .unwrap();

    let draft = Post::owned(1, 1);
    let attributes = ability.attributes_for("create", &draft);
    assert_eq!(attributes.get("user_id"), Some(&DbValue::Int(1)));
    assert!(!attributes.contains_key("id"));
}

#[test]
fn permitted_attributes_add_on_allow_and_remove_on_deny() {
    let mut ability = Ability::new();
    ability
        .can_attributes(
            Some("update"),
            Some("Post"),
            vec!["title".to_owned(), "published".to_owned()],
        )
        .unwrap();
    ability
        .cannot_attributes(Some("update"), Some("Post"), vec!["published".to_owned()])
        .unwrap();

    assert_eq!(
        ability.permitted_attributes("update", "Post"),
        vec!["title".to_owned()]
    );
}

#[test]
fn can_on_attribute_respects_attribute_list() {
    let mut ability = Ability::new();
    ability
        .can_attributes(Some("update"), Some("Post"), vec!["title".to_owned()])
        .unwrap();

    let post = Post::owned(1, 1);
    assert!(ability.can_check_attribute("update", &post, "title"));
    assert!(!ability.can_check_attribute("update", &post, "user_id"));
}

#[test]
fn nested_condition_matches_through_association() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Nested {
                relation: "author".to_owned(),
                condition: Box::new(Condition::Eq {
                    field: "name".to_owned(),
                    value: DbValue::from("ada"),
                }),
            },
        )
        .unwrap();

    let with_author = Post {
        author: Some(User {
            id: 1,
            name: "ada".to_owned(),
        }),
        ..Post::owned(1, 1)
    };
    assert!(ability.can_check("read", &with_author));
    assert!(!ability.can_check("read", &Post::owned(2, 2)));
}

#[test]
fn rules_for_query_rejects_block_rules() {
    let mut ability = Ability::new();
    ability
        .can_matching(Some("read"), Some("Post"), Rc::new(|_| true))
        .unwrap();
    assert_eq!(
        ability.rules_for_query("read", "Post").unwrap_err(),
        CanCanError::BlockInQuery
    );
    assert!(ability.has_matcher("read", "Post"));
}

#[test]
fn rules_for_query_skips_cannot_with_attributes() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Eq {
                field: "user_id".to_owned(),
                value: DbValue::Int(1),
            },
        )
        .unwrap();
    ability
        .cannot_attributes(Some("read"), Some("Post"), vec!["title".to_owned()])
        .unwrap();

    let rules = ability.rules_for_query("read", "Post").unwrap();
    assert_eq!(rules.len(), 1);
    assert!(rules[0].allows());
}

#[test]
fn permissions_report_splits_allow_and_deny() {
    let mut ability = Ability::new();
    ability
        .can_attributes(Some("update"), Some("Post"), vec!["title".to_owned()])
        .unwrap();
    ability.cannot(Some("destroy"), Some("Post")).unwrap();

    let permissions = ability.permissions();
    assert_eq!(
        permissions
            .allowed
            .get("update")
            .and_then(|subjects| subjects.get("Post")),
        Some(&vec!["title".to_owned()])
    );
    assert!(permissions.denied.contains_key("destroy"));
}

#[test]
fn map_subject_covers_dynamic_fields() {
    let mut fields = HashMap::new();
    fields.insert("role".to_owned(), DbValue::from("admin"));
    let subject = MapSubject {
        type_name: "Dashboard",
        fields,
    };

    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Dashboard"),
            Condition::Eq {
                field: "role".to_owned(),
                value: DbValue::from("admin"),
            },
        )
        .unwrap();
    assert!(ability.can_check("read", &subject));
}
