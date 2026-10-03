/// Shared identifier validation used by every query adapter.
///
/// Adapters refuse to render anything whose field or table name breaks this
/// rule, which is the first line of defense against identifier injection
/// through condition field names.
#[must_use]
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|item| item.is_ascii_alphanumeric() || item == '_')
}
