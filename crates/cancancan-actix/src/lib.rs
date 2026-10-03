//! Actix-web integration for `cancancan-core`, mirroring
//! `cancan/controller_additions.rb` from the Ruby gem.
//!
//! Three pieces:
//!
//! * [`CurrentAbility`]: request extractor that pulls the request-scoped
//!   [`Ability`] from request extensions (set by your auth middleware /
//!   `web::Data`) and offers `authorize`, `authorize_type`, `can_check` and
//!   `cannot_check` backed by the core ability.
//! * [`check_authorization`]: `actix_web::middleware::from_fn` handler that
//!   fails requests (500) whose handlers never called `authorize`, mirroring
//!   the gem's `check_authorization`. Opt out per handler with
//!   [`skip_authorization_check`] (mirrors `skip_authorization_check`).
//! * [`AuthorizationError`]: maps [`CanCanError::AccessDenied`] to 403 and
//!   everything else to 500.

use actix_web::body::MessageBody;
use actix_web::dev::{Payload, ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::web::Data;
use actix_web::{Error, FromRequest, HttpMessage, HttpRequest, HttpResponse, ResponseError};
use std::future::{Ready, ready};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cancancan_core::{Ability, CanCanError, SubjectInstance};

/// Shared cell written by [`CurrentAbility::authorize`] and read by the
/// [`check_authorization`] middleware to detect skipped authorization.
#[derive(Debug, Clone, Default)]
struct AuthorizeFlag(Arc<AtomicBool>);

impl AuthorizeFlag {
    fn mark(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    fn was_marked(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Request-scoped ability, mirroring Rails `current_ability`.
#[derive(Debug, Clone)]
pub struct CurrentAbility {
    ability: Ability,
    flag: Option<AuthorizeFlag>,
}

impl CurrentAbility {
    /// The underlying ability.
    #[must_use]
    pub fn ability(&self) -> &Ability {
        &self.ability
    }

    /// Authorizes `action` on `instance`, mirroring Rails `authorize!`.
    ///
    /// Marks the request as authorized (even on denial, so the check
    /// middleware does not report a skip) and returns 403 on failure.
    ///
    /// # Errors
    ///
    /// Returns [`AuthorizationError`] when the underlying check fails
    /// ([`CanCanError::AccessDenied`] —> 403, others —> 500).
    pub fn authorize(
        &self,
        action: &str,
        subject: &dyn SubjectInstance,
    ) -> Result<(), AuthorizationError> {
        let subject_type = subject.subject_type().to_owned();
        self.mark_checked();
        if self.ability.can_check(action, subject) {
            Ok(())
        } else {
            Err(CanCanError::AccessDenied {
                action: action.to_owned(),
                subject: subject_type.clone(),
                message: Some(self.ability.unauthorized_message(action, &subject_type)),
            }
            .into())
        }
    }

    /// Authorizes `action` directly on the subject type name (class-level).
    ///
    /// # Errors
    ///
    /// Same semantics as [`CurrentAbility::authorize`].
    pub fn authorize_type(&self, action: &str, type_name: &str) -> Result<(), AuthorizationError> {
        self.mark_checked();
        let allowed = self.ability.can_check_type(action, type_name);
        if allowed {
            Ok(())
        } else {
            Err(CanCanError::AccessDenied {
                action: action.to_owned(),
                subject: type_name.to_owned(),
                message: Some(self.ability.unauthorized_message(action, type_name)),
            }
            .into())
        }
    }

    /// Mirrors `can?` (no flag marking).
    #[must_use]
    pub fn can_check(&self, action: &str, subject: &dyn SubjectInstance) -> bool {
        self.ability.can_check(action, subject)
    }

    /// Mirrors class-level `can?` (no flag marking).
    #[must_use]
    pub fn can_check_type(&self, action: &str, type_name: &str) -> bool {
        self.ability.can_check_type(action, type_name)
    }

    /// Mirrors `cannot?` (no flag marking).
    #[must_use]
    pub fn cannot_check(&self, action: &str, subject: &dyn SubjectInstance) -> bool {
        self.ability.cannot_check(action, subject)
    }

    fn mark_checked(&self) {
        if let Some(flag) = &self.flag {
            flag.mark();
        }
    }

    /// Opts the current request out of [`check_authorization`].
    ///
    /// Mirrors Rails `skip_authorization_check`.
    pub fn skip_authorization_check(&self) {
        self.mark_checked();
    }
}

impl FromRequest for CurrentAbility {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let (ability, flag) = {
            let extensions = req.extensions();
            let ability = extensions
                .get::<Arc<Ability>>()
                .map(|shared| (**shared).clone())
                .or_else(|| extensions.get::<Ability>().cloned())
                .or_else(|| {
                    extensions
                        .get::<Data<Ability>>()
                        .map(|data| data.get_ref().clone())
                })
                .or_else(|| {
                    req.app_data::<Data<Ability>>()
                        .map(|data| data.get_ref().clone())
                });
            let flag = extensions.get::<AuthorizeFlag>().cloned();
            (ability, flag)
        };
        ready(match ability {
            Some(ability) => Ok(Self { ability, flag }),
            None => Err(AuthorizationError::from(CanCanError::AuthorizationNotPerformed).into()),
        })
    }
}

/// Http error produced by check-time failures and missing wiring.
#[derive(Debug)]
#[non_exhaustive]
pub enum AuthorizationError {
    /// Core authorization error; `AccessDenied` maps to 403, others to 500.
    Core(CanCanError),
}

impl From<CanCanError> for AuthorizationError {
    fn from(error: CanCanError) -> Self {
        Self::Core(error)
    }
}

impl ResponseError for AuthorizationError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        let Self::Core(error) = self;
        match error {
            CanCanError::AccessDenied { .. } => actix_web::http::StatusCode::FORBIDDEN,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let Self::Core(error) = self;
        let body = match error {
            CanCanError::AccessDenied { message, .. } => message
                .clone()
                .unwrap_or_else(|| "Access denied.".to_owned()),
            other => format!("Authorization error: {other}"),
        };
        HttpResponse::build(self.status_code()).body(body)
    }
}

impl std::fmt::Display for AuthorizationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AuthorizationError {}

/// Actix-web middleware entrypoint. Registers the flag and fails skipped
/// requests.
///
/// Use via [`actix_web::middleware::from_fn`]:
///
/// ```rust,ignore
/// let app = App::new().service(..).wrap(middleware::from_fn(check_authorization));
/// ```
///
/// # Errors
///
/// Returns [`CanCanError::AuthorizationNotPerformed`] (as 500) when the inner
/// service ran without a [`CurrentAbility::authorize`] call.
pub async fn check_authorization<B: MessageBody + 'static>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse, Error> {
    let flag = AuthorizeFlag::default();
    req.extensions_mut().insert(flag.clone());
    let response = next.call(req).await;
    match response {
        Ok(response) if flag.was_marked() => Ok(response.map_into_boxed_body()),
        Ok(_skipped) => {
            Err(AuthorizationError::from(CanCanError::AuthorizationNotPerformed).into())
        }
        Err(error) => Err(error),
    }
}
