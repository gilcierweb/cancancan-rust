mod common;

use cancancan_core::{Ability, CanCanError, Condition, DbValue, SubjectRef};
use common::{MapSubject, Post, User};
use std::collections::HashMap;
use std::sync::Arc;

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
    ability.alias_action(["show"], "preview").unwrap();
    ability.can(Some("preview"), Some("Post")).unwrap();

    assert!(ability.can_check_type("show", "Post"));
    assert!(ability.can_check_type("preview", "Post"));
}

#[test]
fn catch_all_cannot_blocks_class_level_check() {
    // gem: cannot matches class-level checks when it carries no conditions
    let mut ability = Ability::new();
    ability.can(Some("read"), Some("all")).unwrap();
    ability.cannot(Some("read"), Some("Post")).unwrap();
    assert!(!ability.can_check_type("read", "Post"));
    assert!(ability.authorize_type("read", "Post").is_err());
    // but instances with guarded can rules still work
    let mut mixed = Ability::new();
    mixed.can(Some("manage"), Some("Post")).unwrap();
    mixed.cannot(Some("destroy"), Some("Post")).unwrap();
    assert!(!mixed.can_check_type("destroy", "Post"));
    assert!(mixed.can_check_type("update", "Post"));
}

#[test]
fn attributes_for_merges_without_evaluating_conditions() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("create"),
            Some("Post"),
            Condition::Eq {
                field: "published".to_owned(),
                value: DbValue::Bool(true),
            },
        )
        .unwrap();
    let post = Post {
        user_id: 0,
        published: false,
        id: 0,
        title: String::new(),
        author: None,
    };
    // gem merges attributes even for rules whose conditions fail on the
    // instance - the failing can rule above still contributes `published`
    let attributes = ability.attributes_for("create", &post);
    assert_eq!(attributes.get("published"), Some(&DbValue::Bool(true)));
}

#[test]
fn range_with_incomparable_bounds_does_not_match() {
    let post = Post {
        user_id: 5,
        published: true,
        id: 0,
        title: String::new(),
        author: None,
    };
    let mut ability = Ability::new();
    // string range against an int field must fail closed, not match
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Range {
                field: "user_id".to_owned(),
                min: DbValue::Str("a".to_owned()),
                max: DbValue::Str("z".to_owned()),
            },
        )
        .unwrap();
    assert!(!ability.can_check("read", &post));
}

#[test]
fn unauthorized_message_walks_alias_chain() {
    let mut ability = Ability::new();
    let resolver: cancancan_core::MessageResolver = std::sync::Arc::new(|action, subject| {
        if action == "manage" && subject == "all" {
            return Some("generic denial".to_owned());
        }
        if action == "read" && subject == "Post" {
            return Some("cannot read posts".to_owned());
        }
        None
    });
    ability.set_message_resolver(resolver);
    // alias: read -> show; exact key wins for the alias target
    assert_eq!(
        ability.unauthorized_message("show", "Post"),
        "cannot read posts"
    );
    // fallback to manage.all
    assert_eq!(
        ability.unauthorized_message("frob", "Post"),
        "generic denial"
    );
}

#[test]
fn merge_copies_aliases_unused_by_rules() {
    let mut other = Ability::new();
    other.alias_action(["modify"], "publish").unwrap();
    // alias registered but no rule references it - merge must still copy it
    let mut ability = Ability::new();
    ability.merge(&other);
    ability.can(Some("publish"), Some("Post")).unwrap();
    let post = Post {
        user_id: 0,
        published: true,
        id: 0,
        title: String::new(),
        author: None,
    };
    assert!(ability.can_check("modify", &post));
}

#[test]
fn permissions_accumulates_attribute_lists() {
    let mut ability = Ability::new();
    ability
        .can_attributes(Some("update"), Some("Post"), vec!["title".to_owned()])
        .unwrap();
    ability
        .can_attributes(Some("update"), Some("Post"), vec!["body".to_owned()])
        .unwrap();
    let permissions = ability.permissions();
    let allowed = permissions
        .allowed
        .get("update")
        .and_then(|by_subject| by_subject.get("Post"))
        .expect("update/Post permissions");
    assert_eq!(allowed, &vec!["title".to_owned(), "body".to_owned()]);
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
            Arc::new(|instance| instance.attribute("published") == Some(DbValue::Bool(true))),
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
fn any_of_matches_when_any_subject_matches() {
    let ability = owner_ability();
    let mine = Post::owned(1, 1);
    let theirs = Post::owned(2, 2);
    assert!(ability.can_check_any_of("read", &[&mine, &theirs]));
    assert!(!ability.can_check_any_of("update", &[&mine, &theirs]));
}

#[test]
fn aliases_for_action_reverse_lookup() {
    let ability = Ability::new();
    assert_eq!(ability.aliases_for_action("show"), vec!["read".to_owned()]);
    assert!(ability.aliases_for_action("destroy").is_empty());
}

#[test]
fn alias_target_colliding_with_mapped_action_is_rejected() {
    let mut ability = Ability::new();
    let error = ability.alias_action(["read"], "show").unwrap_err();
    assert_eq!(error, CanCanError::InvalidAliasTarget("show".to_owned()));
}

#[test]
fn authorize_message_overrides_default() {
    let ability = owner_ability();
    let error = ability
        .authorize_message(
            "read",
            SubjectRef::Instance(&Post::owned(2, 2)),
            "custom denial",
        )
        .unwrap_err();
    assert_eq!(
        error,
        CanCanError::AccessDenied {
            action: "read".to_owned(),
            subject: "Post".to_owned(),
            message: Some("custom denial".to_owned()),
        }
    );
    assert!(
        ability
            .authorize_message("read", SubjectRef::Instance(&Post::owned(1, 1)), "unused")
            .is_ok()
    );
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
    ability.set_message_resolver(Arc::new(|_, _| Some("Not authorized.".to_owned())));
    let error = ability.authorize("read", &Post::owned(2, 2)).unwrap_err();
    assert_eq!(
        error,
        CanCanError::AccessDenied {
            action: "read".to_owned(),
            subject: "Post".to_owned(),
            message: Some("Not authorized.".to_owned()),
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
        .can_matching(Some("read"), Some("Post"), Arc::new(|_| true))
        .unwrap();
    assert_eq!(
        ability.rules_for_query("read", "Post").unwrap_err(),
        CanCanError::BlockInQuery
    );
    assert!(ability.has_matcher("read", "Post"));
}

#[test]
fn aliases_report_and_clear() {
    let mut ability = Ability::new();
    assert!(ability.aliased_actions().contains_key("read"));
    ability.clear_aliased_actions();
    assert!(ability.aliased_actions().is_empty());
    assert!(!ability.can_check_type("show", "Post"));
}

#[test]
fn raw_sql_is_reported() {
    let mut ability = Ability::new();
    assert!(!ability.has_raw_sql("read", "Post"));
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::RawSql("published".to_owned()),
        )
        .unwrap();
    assert!(ability.has_raw_sql("read", "Post"));
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

#[test]
fn can_attributes_where_combines_condition_and_attributes() {
    let mut ability = Ability::new();
    ability
        .can_attributes_where(
            Some("update"),
            Some("Post"),
            vec!["title".to_owned()],
            Condition::Eq {
                field: "user_id".to_owned(),
                value: DbValue::Int(7),
            },
        )
        .unwrap();
    let own = Post {
        id: 1,
        user_id: 7,
        published: false,
        title: String::new(),
        author: None,
    };
    let foreign = Post {
        id: 2,
        user_id: 9,
        published: false,
        title: String::new(),
        author: None,
    };
    assert!(ability.can_check_attribute("update", &own, "title"));
    assert!(!ability.can_check_attribute("update", &own, "published"));
    assert!(!ability.can_check_attribute("update", &foreign, "title"));
    assert_eq!(
        ability.permitted_attributes("update", "Post"),
        vec!["title".to_owned()]
    );
}
