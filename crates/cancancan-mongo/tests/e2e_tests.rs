//! End-to-end proof against a real `mongod` (Docker via testcontainers).

use bson::{Document, doc, oid::ObjectId};
use cancancan_core::{Ability, Condition, DbValue};
use cancancan_mongo::{ColumnMap, ColumnType, accessible_by};
use futures::TryStreamExt;
use mongodb::{Client, Collection};
use std::collections::HashMap;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mongo::Mongo;

const USER_OID: &str = "5c9a3a314c8f4f3a8f9d1a2b";

fn columns() -> ColumnMap {
    HashMap::from([
        ("_id".to_owned(), ColumnType::ObjectId),
        ("user_id".to_owned(), ColumnType::ObjectId),
        ("published".to_owned(), ColumnType::Bool),
        ("title".to_owned(), ColumnType::Text),
    ])
}

fn eq(field: &str, value: DbValue) -> Condition {
    Condition::Eq {
        field: field.to_owned(),
        value,
    }
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
            doc! { "_id": ObjectId::parse_str(USER_OID).unwrap(), "user_id": ObjectId::parse_str(USER_OID).unwrap(), "published": true, "title": "hello" },
            doc! { "user_id": ObjectId::new(), "published": false, "title": "o'brien" },
            doc! { "user_id": ObjectId::parse_str(USER_OID).unwrap(), "published": false, "title": "draft" },
            doc! { "user_id": ObjectId::new(), "published": true, "title": "news" },
        ])
        .await
        .unwrap();
}

async fn visible_titles(collection: &PostCollection, ability: &Ability) -> Vec<String> {
    let filter = accessible_by(ability, "read", "Post", &columns()).unwrap();
    let mut cursor = collection
        .find(filter)
        .sort(doc! { "_id": 1 })
        .await
        .unwrap();
    let mut titles = Vec::new();
    while let Some(post_doc) = cursor.try_next().await.unwrap() {
        titles.push(post_doc.get_str("title").unwrap().to_owned());
    }
    titles
}

#[tokio::test]
async fn owner_reads_own_posts() {
    let (_container, uri) = boot().await;
    let client = Client::with_uri_str(&uri).await.unwrap();
    let collection = client.database("cancancan_test").collection("posts");
    seed(&collection).await;

    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            eq("user_id", DbValue::from(USER_OID)),
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
