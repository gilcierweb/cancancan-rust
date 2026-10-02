/// Builds the default denial message for `action` on `subject_type`.
///
/// Mirrors `UnauthorizedMessageResolver`: consumers override the message per
/// ability through [`crate::Ability::set_message_resolver`].
#[must_use]
pub fn default_message(action: &str, subject_type: &str) -> String {
    format!("You are not authorized to {action} this {subject_type}.")
}
