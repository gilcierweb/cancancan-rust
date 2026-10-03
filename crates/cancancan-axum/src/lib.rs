//! Axum integration for `cancancan-core`, mirroring `cancan/controller_additions.rb`
//! from the Ruby gem.
//!
//! Three pieces:
//!
//! * [`CurrentAbility`]: extractor that pulls the request-scoped [`Ability`]
//!   from request extensions (set by your auth middleware) and offers
//!   `authorize`, `authorize_type`, `can_check` and `cannot_check` backed by
//!   the core ability.
//! * [`check_authorization`]: `axum::middleware::from_fn` handler that fails
//!   requests (500) whose handlers never called `authorize`, mirroring the
//!   gem's `check_authorization`. Opt out per handler with
//!   [`skip_authorization_check`] (mirrors `skip_authorization_check`).
//! * [`AuthorizationError`]: maps [`CanCanError::AccessDenied`] to 403 and
//!   everything else to 500.

use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::{Parts, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

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
///
/// Extracts an [`Ability`] (or `Arc<Ability>`) from request extensions. When
/// the [`check_authorization`] middleware is active, then calling
/// [`CurrentAbility::authorize`] marks the flag it observes.
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
        if self.ability.can_check_type(action, type_name) {
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

impl<S> FromRequestParts<S> for CurrentAbility
where
    S: Send + Sync,
{
    type Rejection = AuthorizationError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // extractor is synchronous by nature; the await only satisfies the
        // async-trait signature
        std::future::ready(()).await;
        let ability = parts
            .extensions
            .get::<Arc<Ability>>()
            .map(|shared| (**shared).clone())
            .or_else(|| parts.extensions.get::<Ability>().cloned())
            .ok_or(CanCanError::AuthorizationNotPerformed)?;
        let flag = parts.extensions.get::<AuthorizeFlag>().cloned();
        Ok(Self { ability, flag })
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

impl IntoResponse for AuthorizationError {
    fn into_response(self) -> Response {
        let Self::Core(error) = self;
        match &error {
            CanCanError::AccessDenied { message, .. } => {
                let body = message
                    .clone()
                    .unwrap_or_else(|| "Access denied.".to_owned());
                (StatusCode::FORBIDDEN, body).into_response()
            }
            _other => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Authorization error: {error}"),
            )
                .into_response(),
        }
    }
}

impl std::fmt::Display for AuthorizationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AuthorizationError {}

/// Axum middleware entrypoint. Registers the flag and fails skipped requests.
///
/// Use via [`axum::middleware::from_fn`]:
///
/// ```rust,ignore
/// let app = Router::new().route(..).route_layer(middleware::from_fn(check_authorization));
/// ```
///
/// Passes the inner response through when the request called
/// [`CurrentAbility::authorize`] or [`CurrentAbility::skip_authorization_check`].
///
/// # Errors
///
/// Returns [`AuthorizationError`] (500) wrapping
/// [`CanCanError::AuthorizationNotPerformed`] when no extractor marked the
/// request as authorized.
pub async fn check_authorization(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthorizationError> {
    let (mut parts, body) = request.into_parts();
    let flag = AuthorizeFlag::default();
    parts.extensions.insert(flag.clone());
    let request = Request::from_parts(parts, body);
    let response = next.run(request).await;
    if flag.was_marked() {
        Ok(response)
    } else {
        Err(AuthorizationError::from(
            CanCanError::AuthorizationNotPerformed,
        ))
    }
}
