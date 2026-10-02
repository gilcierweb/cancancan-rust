use crate::rule::Rule;

/// Removes redundant rules while preserving check semantics.
///
/// Mirrors `CanCan::RulesCompressor`: duplicate conditions are dropped from
/// front to back, then every rule after the last catch-all with the same
/// behavior is discarded (a trailing deny-everything is dropped entirely).
#[must_use]
pub fn compress(rules: Vec<Rule>) -> Vec<Rule> {
    let simplified = simplify(rules);
    let Some(catch_all_index) = simplified.iter().rposition(Rule::is_catch_all) else {
        return simplified;
    };
    let mut compressed: Vec<Rule> = simplified.into_iter().take(catch_all_index + 1).collect();
    let Some(tail) = compressed.pop() else {
        return compressed;
    };
    if tail.is_deny_catch_all() {
        return compressed;
    }
    let behavior = tail.allows();
    compressed.retain(|rule| rule.allows() != behavior || !rule.is_catch_all());
    compressed.push(tail);
    compressed
}

fn simplify(rules: Vec<Rule>) -> Vec<Rule> {
    let mut seen: Vec<Rule> = Vec::with_capacity(rules.len());
    for rule in rules.into_iter().rev() {
        let duplicate = seen.iter().any(|kept| {
            kept.allows() == rule.allows()
                && kept.actions() == rule.actions()
                && kept.subjects() == rule.subjects()
                && kept.condition() == rule.condition()
                && kept.attributes() == rule.attributes()
                && kept.has_matcher() == rule.has_matcher()
        });
        if !duplicate {
            seen.push(rule);
        }
    }
    seen.into_iter().rev().collect()
}
