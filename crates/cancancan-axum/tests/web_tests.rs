use axum::Router;
use axum::body::Body;
use axum::extract::{Extension, Request};
use axum::middleware;

use axum::routing::get;
use cancancan_axum::{CurrentAbility, check_authorization};
use cancancan_core::{Ability, Condition, DbValue};
use std::sync::Arc;
use tower::ServiceExt;

mod subject {
    use cancancan_core::{DbValue, SubjectInstance};

    pub struct Post {
        pub user_id: i64,
        pub published: bool,
    }

    impl Post {
        pub fn owned(user_id: i64) -> Self {
            Self {
                user_id,
                published: false,
            }
        }
    }

    impl SubjectInstance for Post {
        fn subject_type(&self) -> &'static str {
            "Post"
        }

        fn attribute(&self, name: &str) -> Option<DbValue> {
            match name {
                "user_id" => Some(DbValue::Int(self.user_id)),
                "published" => Some(DbValue::Bool(self.published)),
                _ => None,
            }
        }
    }
}

use subject::Post;

fn ability_for(user_id: i64, is_admin: bool) -> Ability {
    let mut ability = Ability::new();
    ability
        .can_where(
            Some("read"),
            Some("Post"),
            Condition::Eq {
                field: "user_id".to_owned(),
                value: DbValue::Int(user_id),
            },
        )
        .unwrap();
    if is_admin {
        ability.can(Some("manage"), Some("all")).unwrap();
    }
    ability
}

async fn read(ability: CurrentAbility) -> Result<&'static str, cancancan_axum::AuthorizationError> {
    ability.authorize("read", &Post::owned(1))?;
    Ok("show")
}

async fn update(
    ability: CurrentAbility,
) -> Result<&'static str, cancancan_axum::AuthorizationError> {
    ability.authorize("update", &Post::owned(1))?;
    Ok("updated")
}

async fn skip(ability: CurrentAbility) -> &'static str {
    ability.skip_authorization_check();
    "skipped"
}

async fn read_pending() -> Result<&'static str, cancancan_axum::AuthorizationError> {
    // Never calls authorize; relies on check_authorization to catch it.
    Ok("fine")
}

async fn load_and_read(
    ability: CurrentAbility,
) -> Result<&'static str, cancancan_axum::AuthorizationError> {
    let post = ability
        .load_and_authorize::<Post, cancancan_axum::AuthorizationError>("read", "Post", async {
            Ok(Some(Post::owned(1)))
        })
        .await?;
    assert!(ability.can_check("read", &post));
    Ok("loaded")
}

async fn load_missing(
    ability: CurrentAbility,
) -> Result<&'static str, cancancan_axum::AuthorizationError> {
    ability
        .load_and_authorize::<Post, cancancan_axum::AuthorizationError>("read", "Post", async {
            Ok(None)
        })
        .await?;
    Ok("unreachable")
}

fn app(ability: Ability) -> Router {
    Router::new()
        .route("/posts/{id}", get(read))
        .route("/posts/{id}/update", get(update))
        .route("/posts/{id}/load", get(load_and_read))
        .route("/posts/{id}/missing", get(load_missing))
        .route("/skip", get(skip))
        .route("/pending", get(read_pending))
        .layer(middleware::from_fn(check_authorization))
        .layer(Extension(Arc::new(ability)))
}

#[tokio::test]
async fn authorized_request_passes() {
    let response = app(ability_for(1, false))
        .oneshot(
            Request::builder()
                .uri("/posts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn denied_request_returns_403() {
    let response = app(ability_for(2, false))
        .oneshot(
            Request::builder()
                .uri("/posts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn authorization_not_performed_returns_500() {
    let response = app(ability_for(1, false))
        .oneshot(
            Request::builder()
                .uri("/pending")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 500);
}

#[tokio::test]
async fn load_and_authorize_marks_request_and_returns_resource() {
    let response = app(ability_for(1, false))
        .oneshot(
            Request::builder()
                .uri("/posts/1/load")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn load_and_authorize_denial_is_403() {
    let response = app(ability_for(2, false))
        .oneshot(
            Request::builder()
                .uri("/posts/1/load")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn load_and_authorize_missing_record_is_404() {
    let response = app(ability_for(1, false))
        .oneshot(
            Request::builder()
                .uri("/posts/1/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
}
