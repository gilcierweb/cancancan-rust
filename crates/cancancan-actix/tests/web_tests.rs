use actix_web::middleware;
use actix_web::{App, Error, web};
use cancancan_actix::{CurrentAbility, check_authorization};
use cancancan_core::{Ability, Condition, DbValue};

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

async fn read(ability: CurrentAbility) -> Result<&'static str, Error> {
    ability.authorize("read", &Post::owned(1))?;
    Ok("show")
}

async fn skip(ability: CurrentAbility) -> Result<&'static str, Error> {
    ability.skip_authorization_check();
    Ok("skipped")
}

async fn pending() -> Result<&'static str, Error> {
    Ok("fine")
}

#[actix_web::test]
async fn authorized_request_passes() {
    let app = actix_web::test::init_service(
        App::new()
            .app_data(web::Data::new(ability_for(1, false)))
            .route("/posts/{id}", web::get().to(read))
            .wrap(middleware::from_fn(check_authorization)),
    )
    .await;
    let req = actix_web::test::TestRequest::get()
        .uri("/posts/1")
        .to_request();
    let response = actix_web::test::call_service(&app, req).await;
    assert_eq!(response.status(), 200);
}

#[actix_web::test]
async fn denied_request_returns_403() {
    let app = actix_web::test::init_service(
        App::new()
            .app_data(web::Data::new(ability_for(2, false)))
            .route("/posts/{id}", web::get().to(read))
            .wrap(middleware::from_fn(check_authorization)),
    )
    .await;
    let req = actix_web::test::TestRequest::get()
        .uri("/posts/1")
        .to_request();
    let response = actix_web::test::call_service(&app, req).await;
    assert_eq!(response.status(), 403);
}

#[actix_web::test]
async fn authorization_not_performed_returns_500() {
    let app = actix_web::test::init_service(
        App::new()
            .app_data(web::Data::new(ability_for(1, false)))
            .route("/pending", web::get().to(pending))
            .wrap(middleware::from_fn(check_authorization)),
    )
    .await;
    let req = actix_web::test::TestRequest::get()
        .uri("/pending")
        .to_request();
    // Errors raised by services are surfaced, producing a 500 error response.
    let error = actix_web::test::try_call_service(&app, req)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_response_error().status_code(),
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[actix_web::test]
async fn skip_authorization_check_allows_unauthorized_route() {
    let app = actix_web::test::init_service(
        App::new()
            .app_data(web::Data::new(ability_for(1, false)))
            .route("/skip", web::get().to(skip))
            .wrap(middleware::from_fn(check_authorization)),
    )
    .await;
    let req = actix_web::test::TestRequest::get()
        .uri("/skip")
        .to_request();
    let response = actix_web::test::call_service(&app, req).await;
    assert_eq!(response.status(), 200);
}
