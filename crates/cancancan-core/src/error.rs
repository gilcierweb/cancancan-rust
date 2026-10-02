use thiserror::Error;

/// Errors raised by authorization checks and rule definitions.
///
/// Mirrors `lib/cancan/exceptions.rb` from the Ruby gem.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanCanError {
    /// Raised by [`crate::Ability::authorize`] when the check fails.
    #[error("access denied: cannot {action} {subject}")]
    AccessDenied {
        /// Action that was attempted (e.g. `"read"`).
        action: String,
        /// Subject type the action was attempted on (e.g. `"Post"`).
        subject: String,
        /// Resolved human-readable message, if any.
        message: Option<String>,
    },

    /// Raised when a controller layer requires authorization but none ran.
    #[error("authorization has not been performed")]
    AuthorizationNotPerformed,

    /// Raised when a rule combines hash conditions with a block matcher.
    #[error("a rule cannot combine conditions with a block matcher")]
    BlockAndConditions,

    /// Raised when the attributes argument of a rule is not a valid attribute list.
    #[error("invalid attribute argument for rule definition")]
    AttributeArgument,

    /// Raised when an action is given without a subject.
    #[error("an action cannot be defined without a subject")]
    ActionWithoutSubject,

    /// Raised when block-only rules are used where a query is required.
    #[error("rules with a block matcher cannot be used for database queries")]
    BlockInQuery,

    /// Raised when a nested condition references an unknown association.
    #[error("unknown association in conditions: {0}")]
    WrongAssociation(String),
}
