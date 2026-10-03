//! End-to-end proof against a real `mongod` (Docker via testcontainers).
//!
//! Documents store `user_id` as native BSON binary UUID (subtype 4), matching
//! the driver's default serialization, and exercise `archived_by` as a
//! nullable link present-null vs fully absent.

use bson::{
    Binary, Document, doc,
    oid::ObjectId,
    uuid::{Uuid as BsonUuid, UuidRepresentation},
};
use cancancan_core::{Ability, Condition, DbValue};
use cancancan_mongo::{ColumnMap, ColumnType, accessible_by};
use futures::TryStreamExt;
use mongodb::{Client, Collection};
use std::collections::HashMap;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mongo::Mongo;

const USER_OID: &str = "5c9a3a314c8f4f3a8f9d1a2b";
const USER_UUID: &str = "5c9a3a31-4c8f-4f3a-8f9d-1a2b3c4d5e6f";
const OTHER_UUID: &str = "f47ac10b-58cc-4372-a567-0e02b2c3d479";

fn columns() -> ColumnMap {
    HashMap::from([
        ("_id".to_owned(), ColumnType::ObjectId),
        ("user_id".to_owned(), ColumnType::Uuid),
        ("published".to_owned(), ColumnType::Bool),
        ("title".to_owned(), ColumnType::Text),
        ("archived_by".to_owned(), ColumnType::ObjectId),
    ])
}

fn eq(field: &str, value: DbValue) -> Condition {
    Condition::Eq {
        field: field.to_owned(),
        value,
    }
}

fn uuid_binary(uuid_str: &str) -> Binary {
    Binary::from_uuid_with_representation(
        BsonUuid::from(uuid::Uuid::parse_str(uuid_str).unwrap()),
        UuidRepresentation::Standard,
    )
}

async fn boot() -> (testcontainers::ContainerAsync<Mongo>, String) {
    let container = Mongo::default()
        .start()
        .await
        .expect("docker must be available");
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(27017).await.unwrap();
    let uri = format!("mongodb://{host}:{port}/?directConnection=true");
    (container, uri)
}

type PostCollection = Collection<Document>;

async fn seed(collection: &PostCollection) {
    collection
        .insert_many([
            // owner, native binary UUID FK, archived_by present and null
            doc! {
                "_id": ObjectId::parse_str(USER_OID).unwrap(),
                "user_id": uuid_binary(USER_UUID),
                "published": true,
                "title": "hello",
                "archived_by": bson::Bson::Null,
            },
            // other user, archived_by absent entirely
            doc! {
                "user_id": uuid_binary(OTHER_UUID),
                "published": false,
                "title": "o'brien",
            },
            // owner, archived_by set to a real ObjectId
            doc! {
                "user_id": uuid_binary(USER_UUID),
                "published": false,
                "title": "draft",
                "archived_by": ObjectId::new(),
            },
            // other user, no archived_by
            doc! {
                "user_id": uuid_binary(OTHER_UUID),
                "published": true,
                "title": "news",
            },
        ])
        .await
        .unwrap();
}

async fn visible_titles(collection: &PostCollection, ability: &Ability) -> Vec<String> {
    let filter = accessible_by(ability, "read", "Post", &columns()).unwrap();
    let mut curs = collection
        .find(filter)
        .sort(doc! { "_id": 1, "title": 1 })
        .await
        .unwrap();
    let mut titles = Vec::new();
    while let Some(post_doc) = curs.try_next().await.unwrap() {
        titles.push(post_doc.get_str("title").unwrap().to_owned());
    }
    titles
}

#[tokio::test]
async fn owner_reads_own_posts_via_native_binary_uuid() {
    let (_container, uri) = boot().await;
    let client = Client::with_uri_str(&uri).await.unwrap();
    let collection = client.database("cancancan_test").collection("posts");
    seed(&collection).await;

    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("user_id", DbValue::from(USER_UUID)),
        )
        .unwrap();
    assert_eq!(
        visible_titles(&collection, &ability).await,
        vec!["hello".to_owned(), "draft".to_owned()]
    );
}

#[tokio::test]
async fn deny_removes_unpublished_posts() {
    let (_container, uri) = boot().await;
    let client = Client::with_uri_str(&uri).await.unwrap();
    let collection = client.database("cancancan_test").collection("posts");
    seed(&collection).await;

    let mut ability = Ability::new();
    ability.can(Some("read"), Some("Post")).unwrap();
    ability
        .cannot_where(
            Some("read"),
            Some("Post"),
            eq("published", DbValue::Bool(false)),
        )
        .unwrap();
    assert_eq!(
        visible_titles(&collection, &ability).await,
        vec!["hello".to_owned(), "news".to_owned()]
    );
}

#[tokio::test]
async fn strict_is_null_matches_only_present_null() {
    let (_container, uri) = boot().await;
    let client = Client::with_uri_str(&uri).await.unwrap();
    let collection = client.database("cancancan_test").collection("posts");
    seed(&collection).await;

    let mut ability = Ability::new();
    ability
        .can_where(Some("read"), Some("Post"), eq("archived_by", DbValue::Null))
        .unwrap();
    assert_eq!(
        visible_titles(&collection, &ability).await,
        vec!["hello".to_owned()]
    );
}

#[tokio::test]
async fn strict_not_null_matches_only_present_non_null() {
    let (_container, uri) = boot().await;
    let client = Client::with_uri_str(&uri).await.unwrap();
    let collection = client.database("cancancan_test").collection("posts");
    seed(&collection).await;

    // positive form: only documents with a present, non-null archived_by
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Ne {
                field: "archived_by".to_owned(),
                value: DbValue::Null,
            },
        )
        .unwrap();
    let mut titles = visible_titles(&collection, &ability).await;
    titles.sort();
    assert_eq!(titles, vec!["draft".to_owned()]);

    // negated form: $nor of strict-null excludes just the explicit-null doc
    let mut ability = Ability::new();
    ability.can(Some("read"), Some("Post")).unwrap();
    ability
        .cannot_where(Some("read"), Some("Post"), eq("archived_by", DbValue::Null))
        .unwrap();
    let mut titles = visible_titles(&collection, &ability).await;
    titles.sort();
    assert_eq!(
        titles,
        vec!["draft".to_owned(), "news".to_owned(), "o'brien".to_owned()]
    );
}

#[tokio::test]
async fn no_rules_match_nothing() {
    let (_container, uri) = boot().await;
    let client = Client::with_uri_str(&uri).await.unwrap();
    let collection = client.database("cancancan_test").collection("posts");
    seed(&collection).await;

    assert_eq!(
        visible_titles(&collection, &Ability::new()).await,
        Vec::<String>::new()
    );
}

#[tokio::test]
async fn catch_all_matches_everything() {
    let (_container, uri) = boot().await;
    let client = Client::with_uri_str(&uri).await.unwrap();
    let collection = client.database("cancancan_test").collection("posts");
    seed(&collection).await;

    let mut ability = Ability::new();
    ability.can(Some("manage"), Some("all")).unwrap();
    assert_eq!(visible_titles(&collection, &ability).await.len(), 4);
}

#[tokio::test]
async fn quoted_title_matches() {
    let (_container, uri) = boot().await;
    let client = Client::with_uri_str(&uri).await.unwrap();
    let collection = client.database("cancancan_test").collection("posts");
    seed(&collection).await;

    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("title", DbValue::from("o'brien")),
        )
        .unwrap();
    assert_eq!(
        visible_titles(&collection, &ability).await,
        vec!["o'brien".to_owned()]
    );
}
