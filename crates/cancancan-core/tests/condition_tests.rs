mod common;

use cancancan_core::{Condition, DbValue};
use common::{MapSubject, Post};
use rstest::rstest;
use std::collections::HashMap;

fn subject_with(fields: &[(&str, DbValue)]) -> MapSubject {
    MapSubject {
        type_name: "Post",
        fields: fields
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect(),
    }
}

#[rstest]
#[case::eq_match(
    Condition::Eq { field: "user_id".to_owned(), value: DbValue::Int(1) },
    &[("user_id", DbValue::Int(1))],
    true,
)]
#[case::eq_mismatch(
    Condition::Eq { field: "user_id".to_owned(), value: DbValue::Int(1) },
    &[("user_id", DbValue::Int(2))],
    false,
)]
#[case::eq_missing_field(
    Condition::Eq { field: "user_id".to_owned(), value: DbValue::Int(1) },
    &[],
    false,
)]
#[case::ne_match(
    Condition::Ne { field: "user_id".to_owned(), value: DbValue::Int(1) },
    &[("user_id", DbValue::Int(2))],
    true,
)]
#[case::ne_same_value(
    Condition::Ne { field: "user_id".to_owned(), value: DbValue::Int(1) },
    &[("user_id", DbValue::Int(1))],
    false,
)]
#[case::in_match(
    Condition::In { field: "id".to_owned(), values: vec![DbValue::Int(1), DbValue::Int(2)] },
    &[("id", DbValue::Int(2))],
    true,
)]
#[case::in_mismatch(
    Condition::In { field: "id".to_owned(), values: vec![DbValue::Int(1), DbValue::Int(2)] },
    &[("id", DbValue::Int(9))],
    false,
)]
#[case::in_empty_never_matches(
    Condition::In { field: "id".to_owned(), values: vec![] },
    &[("id", DbValue::Int(1))],
    false,
)]
#[case::range_inside(
    Condition::Range { field: "id".to_owned(), min: DbValue::Int(1), max: DbValue::Int(10) },
    &[("id", DbValue::Int(5))],
    true,
)]
#[case::range_boundary_inclusive(
    Condition::Range { field: "id".to_owned(), min: DbValue::Int(1), max: DbValue::Int(10) },
    &[("id", DbValue::Int(10))],
    true,
)]
#[case::range_outside(
    Condition::Range { field: "id".to_owned(), min: DbValue::Int(1), max: DbValue::Int(10) },
    &[("id", DbValue::Int(11))],
    false,
)]
#[case::is_null_when_missing(
    Condition::IsNull { field: "title".to_owned(), is_null: true },
    &[],
    true,
)]
#[case::is_null_when_present(
    Condition::IsNull { field: "title".to_owned(), is_null: true },
    &[("title", DbValue::from("hi"))],
    false,
)]
#[case::not_null_when_present(
    Condition::IsNull { field: "title".to_owned(), is_null: false },
    &[("title", DbValue::from("hi"))],
    true,
)]
fn condition_matches_cases(
    #[case] condition: Condition,
    #[case] fields: &[(&str, DbValue)],
    #[case] expected: bool,
) {
    assert_eq!(condition.matches(&subject_with(fields)), expected);
}

#[test]
fn and_or_not_combinators() {
    let user_id_is_one = || Condition::Eq {
        field: "user_id".to_owned(),
        value: DbValue::Int(1),
    };
    let published = || Condition::Eq {
        field: "published".to_owned(),
        value: DbValue::Bool(true),
    };

    let both = Condition::And(vec![user_id_is_one(), published()]);
    assert!(both.matches(&Post {
        published: true,
        ..Post::owned(1, 1)
    }));
    assert!(!both.matches(&Post {
        published: false,
        ..Post::owned(1, 1)
    }));

    let either = Condition::Or(vec![user_id_is_one(), published()]);
    assert!(either.matches(&Post {
        published: true,
        ..Post::owned(9, 9)
    }));
    assert!(!either.matches(&Post {
        published: false,
        ..Post::owned(9, 9)
    }));

    let negated = Condition::Not(Box::new(user_id_is_one()));
    assert!(negated.matches(&Post::owned(1, 9)));
    assert!(!negated.matches(&Post::owned(1, 1)));

    assert!(Condition::And(vec![]).matches(&Post::owned(1, 1)));
    assert!(!Condition::Or(vec![]).matches(&Post::owned(1, 1)));
    assert!(Condition::All.matches(&Post::owned(1, 1)));
}

#[test]
fn raw_sql_never_matches_in_memory() {
    let condition = Condition::RawSql("published = TRUE".to_owned());
    assert!(!condition.matches(&Post::owned(1, 1)));
    assert!(condition.scalar_attributes().is_empty());
}

#[test]
fn scalar_attributes_collects_only_plain_equalities() {
    let condition = Condition::And(vec![
        Condition::Eq {
            field: "user_id".to_owned(),
            value: DbValue::Int(1),
        },
        Condition::In {
            field: "id".to_owned(),
            values: vec![DbValue::Int(1)],
        },
        Condition::Range {
            field: "score".to_owned(),
            min: DbValue::Int(1),
            max: DbValue::Int(5),
        },
    ]);
    let attributes = condition.scalar_attributes();
    assert_eq!(
        attributes,
        HashMap::from([("user_id".to_owned(), DbValue::Int(1))])
    );
}
