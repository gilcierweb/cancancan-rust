use crate::condition::Condition;
use crate::rule::Rule;

/// Removes redundant rules while preserving check semantics.
///
/// Mirrors `CanCan::RulesCompressor`. The input is the adapter-facing rule
/// list (relevant rules, **latest-defined first**, as produced by
/// [`crate::Ability::rules_for_query`]); the gem receives the same ordering
/// (`@rules.reverse` of its relevant list), so the algorithm runs on the
/// definition-ordered copy and the result is reversed back.
///
/// Steps, mirroring the gem: duplicate rules are dropped (same condition,
/// keeping the latest definition), then the rule list is truncated at the
/// latest catch-all, keeping only later rules of the opposite behavior - a
/// trailing deny-everything replaces the set entirely.
#[must_use]
pub fn compress(rules: Vec<Rule>) -> Vec<Rule> {
    let definition_order: Vec<Rule> = rules.into_iter().rev().collect();
    let definition_order = simplify(definition_order);

    let Some(catch_all_index) = definition_order.iter().rposition(Rule::is_catch_all) else {
        return definition_order.into_iter().rev().collect();
    };

    let mut definition_order = definition_order;
    // Rules defined after the catch-all, minus the leading run of rules with
    // the same behavior (the catch-all already covers them). Rules defined
    // before the catch-all are unreachable and dropped.
    let catch_all = definition_order[catch_all_index].clone();
    let tail = definition_order.split_off(catch_all_index + 1);
    let keep_from = tail
        .iter()
        .position(|rule| rule.allows() != catch_all.allows())
        .unwrap_or(tail.len());
    let mut kept: Vec<Rule> = tail.into_iter().skip(keep_from).collect();
    let mut compressed: Vec<Rule> = Vec::new();
    if !catch_all.is_deny_catch_all() {
        compressed.push(catch_all);
    }
    compressed.append(&mut kept);
    compressed.into_iter().rev().collect()
}

/// Drops every rule whose condition duplicates a later rule's, mirroring the
/// gem's `simplify` (`A OR (A OR x)` collapses to `A OR x` regardless of
/// behavior).
fn simplify(rules: Vec<Rule>) -> Vec<Rule> {
    let mut seen: Vec<Condition> = Vec::with_capacity(rules.len());
    rules
        .into_iter()
        .rev()
        .filter(|rule| {
            if seen.contains(rule.condition()) {
                false
            } else {
                seen.push(rule.condition().clone());
                true
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
