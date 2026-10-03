use bson::{Document, doc, oid::ObjectId};
use cancancan_core::{Ability, CanCanError, Condition, DbValue};
use cancancan_mongo::{ColumnMap, ColumnType, accessible_by, condition_to_doc};
use std::collections::HashMap;
use std::rc::Rc;

const USER_OID: &str = "5c9a3a314c8f4f3a8f9d1a2b";
const USER_UUID: &str = "5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f";

fn columns() -> ColumnMap {
    HashMap::from([
        ("_id".to_owned(), ColumnType::ObjectId),
        ("user_id".to_owned(), ColumnType::Uuid),
        ("legacy_id".to_owned(), ColumnType::Long),
        ("published".to_owned(), ColumnType::Bool),
        ("title".to_owned(), ColumnType::Text),
        ("views".to_owned(), ColumnType::Int),
        ("score".to_owned(), ColumnType::Double),
    ])
}

fn eq(field: &str, value: DbValue) -> Condition {
    Condition::Eq {
        field: field.to_owned(),
        value,
    }
}

#[test]
fn object_id_parsed_and_bound_natively() {
    let filter = condition_to_doc(&eq("_id", DbValue::from(USER_OID)), &columns()).unwrap();
    let expected_id = ObjectId::parse_str(USER_OID).unwrap();
    assert_eq!(filter, doc! { "_id": expected_id });
}

#[test]
fn uuid_validated_and_bound_as_canonical_text() {
    let filter = condition_to_doc(&eq("user_id", DbValue::from(USER_UUID)), &columns()).unwrap();
    assert_eq!(filter, doc! { "user_id": USER_UUID });

    let error =
        condition_to_doc(&eq("user_id", DbValue::from("not-a-uuid")), &columns()).unwrap_err();
    assert_eq!(error, CanCanError::AttributeArgument);
}

#[test]
fn int_bound_types_follow_column_width() {
    let filter = condition_to_doc(&eq("legacy_id", DbValue::Int(9)), &columns()).unwrap();
    assert_eq!(filter, doc! { "legacy_id": bson::Bson::Int64(9) });

    let filter = condition_to_doc(&eq("views", DbValue::Int(9)), &columns()).unwrap();
    assert_eq!(filter, doc! { "views": bson::Bson::Int32(9) });

    let overflow = condition_to_doc(
        &eq("views", DbValue::Int(i64::from(i32::MAX) + 1)),
        &columns(),
    );
    assert_eq!(overflow.unwrap_err(), CanCanError::AttributeArgument);
}

#[test]
fn eq_null_ne_null() {
    let filter = condition_to_doc(&eq("title", DbValue::Null), &columns()).unwrap();
    assert_eq!(filter, doc! { "title": bson::Bson::Null });

    let filter = condition_to_doc(
        &Condition::Ne {
            field: "title".to_owned(),
            value: DbValue::Null,
        },
        &columns(),
    )
    .unwrap();
    assert_eq!(filter, doc! { "title": { "$ne": bson::Bson::Null } });
}

#[test]
fn in_ne_range() {
    let filter = condition_to_doc(
        &Condition::In {
            field: "legacy_id".to_owned(),
            values: vec![DbValue::Int(1), DbValue::Int(2)],
        },
        &columns(),
    )
    .unwrap();
    assert_eq!(
        filter,
        doc! { "legacy_id": { "$in": [bson::Bson::Int64(1), bson::Bson::Int64(2)] } }
    );

    let filter = condition_to_doc(
        &Condition::In {
            field: "legacy_id".to_owned(),
            values: vec![],
        },
        &columns(),
    )
    .unwrap();
    assert_eq!(filter, doc! { "legacy_id": { "$in": bson::Array::new() } });

    let filter = condition_to_doc(
        &Condition::Range {
            field: "score".to_owned(),
            min: DbValue::Float(1.5),
            max: DbValue::Float(9.5),
        },
        &columns(),
    )
    .unwrap();
    assert_eq!(
        filter,
        doc! { "score": { "$gte": 1.5_f64, "$lte": 9.5_f64 } }
    );
}

#[test]
fn and_or_not_shape() {
    let filter = condition_to_doc(
        &Condition::And(vec![
            eq("published", DbValue::Bool(true)),
            Condition::Range {
                field: "legacy_id".to_owned(),
                min: DbValue::Int(1),
                max: DbValue::Int(10),
            },
        ]),
        &columns(),
    )
    .unwrap();
    assert_eq!(
        filter,
        doc! {
            "$and": [
                { "published": true },
                { "legacy_id": { "$gte": bson::Bson::Int64(1), "$lte": bson::Bson::Int64(10) } },
            ]
        }
    );

    let filter = condition_to_doc(
        &Condition::Or(vec![
            eq("published", DbValue::Bool(true)),
            eq("title", DbValue::Null),
        ]),
        &columns(),
    )
    .unwrap();
    assert_eq!(
        filter,
        doc! { "$or": [ { "published": true }, { "title": bson::Bson::Null } ] }
    );

    let filter = condition_to_doc(
        &Condition::Not(Box::new(eq("published", DbValue::Bool(false)))),
        &columns(),
    )
    .unwrap();
    assert_eq!(filter, doc! { "$nor": [ { "published": false } ] });
}

#[test]
fn nested_flattens_to_dot_notation() {
    let filter = condition_to_doc(
        &Condition::Nested {
            relation: "author".to_owned(),
            condition: Box::new(Condition::Nested {
                relation: "profile".to_owned(),
                condition: Box::new(eq("title", DbValue::from("ada"))),
            }),
        },
        &columns(),
    )
    .unwrap();
    assert_eq!(filter, doc! { "author.profile.title": "ada" });
}

#[test]
fn nested_invalid_relation_rejected() {
    let error = condition_to_doc(
        &Condition::Nested {
            relation: "a.b".to_owned(),
            condition: Box::new(eq("title", DbValue::from("t"))),
        },
        &columns(),
    )
    .unwrap_err();
    assert_eq!(error, CanCanError::WrongAssociation("a.b".to_owned()));
}

#[test]
fn raw_sql_is_rejected() {
    let error =
        condition_to_doc(&Condition::RawSql("published".to_owned()), &columns()).unwrap_err();
    assert_eq!(error, CanCanError::RawSqlNotSupported("mongodb"));
}

#[test]
fn unknown_field_rejected() {
    let error = condition_to_doc(&eq("missing", DbValue::Int(1)), &columns()).unwrap_err();
    assert_eq!(error, CanCanError::AttributeArgument);
}

#[test]
fn accessible_by_composes_or_equals_and_nor() {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("user_id", DbValue::from(USER_UUID)),
        )
        .unwrap();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("published", DbValue::Bool(true)),
        )
        .unwrap();
    ability
        .cannot_where(Some("read"), Some("Post"), eq("legacy_id", DbValue::Int(9)))
        .unwrap();

    let filter = accessible_by(&ability, "read", "Post", &columns()).unwrap();
    assert_eq!(
        filter,
        doc! {
            "$or": [
                { "published": true },
                { "user_id": USER_UUID },
            ],
            "$nor": [
                { "legacy_id": bson::Bson::Int64(9) },
            ],
        }
    );
}

#[test]
fn accessible_by_no_rules_matches_nothing() {
    let filter = accessible_by(&Ability::new(), "read", "Post", &columns()).unwrap();
    assert_eq!(filter, doc! { "$expr": false });
}

#[test]
fn accessible_by_catch_all_matches_everything() {
    let mut ability = Ability::new();
    ability.can(Some("manage"), Some("all")).unwrap();
    let filter = accessible_by(&ability, "read", "Post", &columns()).unwrap();
    assert_eq!(filter, Document::new());
}

#[test]
fn accessible_by_matcher_rule_rejected() {
    let mut ability = Ability::new();
    ability
        .can_matching(Some("read"), Some("Post"), Rc::new(|_| true))
        .unwrap();
    assert_eq!(
        accessible_by(&ability, "read", "Post", &columns()).unwrap_err(),
        CanCanError::BlockInQuery
    );
}
