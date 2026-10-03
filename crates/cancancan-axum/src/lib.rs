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
//!   [`skip_authorization_check`](CurrentAbility::skip_authorization_check)
//!   (mirrors `skip_authorization_check`).
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

    /// Authorizes the instance (mirrors `authorize_resource`; only marks the
    /// request, no loading).
    ///
    /// # Errors
    ///
    /// Same semantics as [`CurrentAbility::authorize`].
    pub fn authorize_resource(
        &self,
        action: &str,
        subject: &dyn SubjectInstance,
    ) -> Result<(), AuthorizationError> {
        self.authorize(action, subject)
    }

    /// Loads `subject` with `loader`, then authorizes it (mirrors
    /// `load_and_authorize_resource`).
    ///
    /// The `loader` returns `Ok(Some(subject))` when found, `Ok(None)` when
    /// not found (404), or an error (500).
    ///
    /// # Errors
    ///
    /// Returns [`AuthorizationError`] on permission failure (403), missing
    /// record (404) or loader failure (500).
    pub async fn load_and_authorize<T, E>(
        &self,
        action: &str,
        subject_type: &str,
        loader: impl std::future::Future<Output = Result<Option<T>, E>>,
    ) -> Result<T, AuthorizationError>
    where
        T: SubjectInstance,
        E: std::fmt::Display,
    {
        let _ = subject_type; // only used by callers for the error message context
        let subject = self.load_resource(loader).await?;
        self.authorize(action, &subject)?;
        Ok(subject)
    }

    /// Loads with `loader` without authorizing (mirrors `load_resource`).
    ///
    /// # Errors
    ///
    /// Returns [`AuthorizationError::NotFound`] (404) when `loader` yields
    /// `None`, or [`AuthorizationError::LoadFailed`] (500) when the loader
    /// errors.
    pub async fn load_resource<T, E>(
        &self,
        loader: impl std::future::Future<Output = Result<Option<T>, E>>,
    ) -> Result<T, AuthorizationError>
    where
        T: SubjectInstance,
        E: std::fmt::Display,
    {
        loader
            .await
            .map_err(|error| AuthorizationError::LoadFailed(error.to_string()))?
            .ok_or_else(|| AuthorizationError::NotFound("record not found".to_owned()))
    } // closes load_resource

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
    /// Record not found by the loader (404).
    NotFound(String),
    /// Loader (database, remote call) failed (500).
    LoadFailed(String),
}

impl From<CanCanError> for AuthorizationError {
    fn from(error: CanCanError) -> Self {
        Self::Core(error)
    }
}

impl IntoResponse for AuthorizationError {
    fn into_response(self) -> Response {
        match self {
            Self::Core(CanCanError::AccessDenied { message, .. }) => {
                let body = message.unwrap_or_else(|| "Access denied.".to_owned());
                (StatusCode::FORBIDDEN, body).into_response()
            }
            Self::Core(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Authorization error: {error}"),
            )
                .into_response(),
            Self::NotFound(body) => (StatusCode::NOT_FOUND, body).into_response(),
            Self::LoadFailed(reason) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Load failed: {reason}"),
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
/// [`CanCanError::AuthorizationNotPerformed`] when a **successful** handler
/// ran without calling [`CurrentAbility::authorize`] or
/// [`CurrentAbility::skip_authorization_check`]. Error responses (4xx/5xx)
/// pass through unchanged, since an erroring handler never completes an
/// authorization decision.
pub async fn check_authorization(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthorizationError> {
    let (mut parts, body) = request.into_parts();
    let flag = AuthorizeFlag::default();
    parts.extensions.insert(flag.clone());
    let request = Request::from_parts(parts, body);
    let response = next.run(request).await;
    if flag.was_marked()
        || response.status().is_client_error()
        || response.status().is_server_error()
    {
        Ok(response)
    } else {
        Err(AuthorizationError::from(
            CanCanError::AuthorizationNotPerformed,
        ))
    }
}
