use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::Condition;
use crate::actions::Actions;
use crate::condition::{DbValue, SubjectInstance};
use crate::error::CanCanError;
use crate::messages::default_message;
use crate::rule::{BlockMatcher, Rule};

/// Subject an ability check runs against.
#[derive(Clone, Copy)]
pub enum SubjectRef<'subject> {
    /// Check against a subject type name (e.g. `"Post"`), mirroring class checks.
    Type(&'subject str),
    /// Check against a concrete instance, mirroring instance checks.
    Instance(&'subject dyn SubjectInstance),
}

impl std::fmt::Debug for SubjectRef<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Type(name) => formatter.debug_tuple("Type").field(name).finish(),
            Self::Instance(instance) => formatter
                .debug_tuple("Instance")
                .field(&instance.subject_type())
                .finish(),
        }
    }
}

/// Custom resolver for denial messages, mirroring the i18n lookup.
pub type MessageResolver = Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>;

/// Defines and checks authorization rules.
///
/// Mirrors `CanCan::Ability`: rules are declared with `can`/`cannot`
/// (plus `_where`, `_matching` and `_attributes` variants), checked with
/// [`Ability::can_check`] (`!can_check` and the `cannot_check` family cover
/// `cannot?`, [`Ability::can_check_type`] covers class-level checks) and
/// enforced with [`Ability::authorize`]. The last matching rule wins.
///
/// # Example
///
/// ```rust
/// use cancancan_core::{Ability, Condition, SubjectInstance, DbValue};
///
/// struct Post { user_id: i64 }
///
/// impl SubjectInstance for Post {
///     fn subject_type(&self) -> &'static str { "Post" }
///     fn attribute(&self, name: &str) -> Option<DbValue> {
///         if name == "user_id" { Some(DbValue::Int(self.user_id)) } else { None }
///     }
/// }
///
/// let mut ability = Ability::new();
/// ability.can_where(Some("read"), Some("Post"), Condition::Eq {
///     field: "user_id".to_owned(),
///     value: DbValue::Int(1),
/// }).unwrap();
///
/// assert!(ability.can_check("read", &Post { user_id: 1 }));
/// assert!(ability.cannot_check("read", &Post { user_id: 2 }));
/// ```
#[derive(Clone)]
pub struct Ability {
    rules: Vec<Rule>,
    actions: Actions,
    message_resolver: Option<MessageResolver>,
}

impl Ability {
    /// Creates an ability without rules and with default action aliases.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            actions: Actions::new(),
            message_resolver: None,
        }
    }

    /// Registers an alias, so checking the alias also checks every mapped action.
    ///
    /// Mirrors the gem `alias_action`, including its `validate_target` guard.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::InvalidAliasTarget`] when `target` is already
    /// mapped as a concrete action of another alias.
    pub fn alias_action<I, S>(&mut self, actions: I, target: S) -> Result<&mut Self, CanCanError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let target = target.into();
        if self.actions.collides_with_mapping(&target) {
            return Err(CanCanError::InvalidAliasTarget(target));
        }
        let mapped: Vec<String> = actions.into_iter().map(Into::into).collect();
        self.actions.alias_action(mapped, target);
        Ok(self)
    }

    /// Removes every action alias, including the defaults.
    ///
    /// Mirrors `clear_aliased_actions`.
    pub fn clear_aliased_actions(&mut self) -> &mut Self {
        self.actions.clear();
        self
    }

    /// Returns every registered alias target with its mapped actions.
    ///
    /// Mirrors `aliased_actions`.
    #[must_use]
    pub fn aliased_actions(&self) -> std::collections::HashMap<String, Vec<String>> {
        self.actions.aliases()
    }

    /// Overrides how denial messages are resolved.
    pub fn set_message_resolver(&mut self, resolver: MessageResolver) -> &mut Self {
        self.message_resolver = Some(resolver);
        self
    }

    /// Declares a `can` rule for `action` on `subject_type`.
    ///
    /// Pass `None` for either argument to match everything, mirroring
    /// `can` without arguments.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn can(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::can(action.map(str::to_owned), subject_type.map(str::to_owned))?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `cannot` rule for `action` on `subject_type`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn cannot(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::cannot(action.map(str::to_owned), subject_type.map(str::to_owned))?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `can` rule guarded by a declarative `condition`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn can_where(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        condition: Condition,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::can_where(
            action.map(str::to_owned),
            subject_type.map(str::to_owned),
            condition,
        )?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `cannot` rule guarded by a declarative `condition`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn cannot_where(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        condition: Condition,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::cannot_where(
            action.map(str::to_owned),
            subject_type.map(str::to_owned),
            condition,
        )?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `can` rule evaluated by `matcher` at check time.
    ///
    /// Matcher rules only run in memory; adapters reject them for queries.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn can_matching(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        matcher: BlockMatcher,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::can_matching(
            action.map(str::to_owned),
            subject_type.map(str::to_owned),
            matcher,
        )?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `cannot` rule evaluated by `matcher` at check time.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn cannot_matching(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        matcher: BlockMatcher,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::cannot_matching(
            action.map(str::to_owned),
            subject_type.map(str::to_owned),
            matcher,
        )?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `can` rule exposing `attributes` for parameter filtering.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn can_attributes(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        attributes: Vec<String>,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::can(action.map(str::to_owned), subject_type.map(str::to_owned))?
            .with_attributes(attributes);
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `cannot` rule exposing `attributes` for parameter filtering.
    ///
    /// `cannot` rules remove names previously added by `can` rules in
    /// [`Ability::permitted_attributes`].
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn cannot_attributes(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        attributes: Vec<String>,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::cannot(action.map(str::to_owned), subject_type.map(str::to_owned))?
            .with_attributes(attributes);
        self.rules.push(rule);
        Ok(self)
    }

    /// Checks whether `action` is permitted on `instance`.
    ///
    /// This is the `can?` equivalent. Concrete references coerce
    /// automatically: `ability.can_check("read", &post)`.
    #[must_use]
    pub fn can_check(&self, action: &str, instance: &dyn SubjectInstance) -> bool {
        self.evaluate(action, SubjectRef::Instance(instance), None)
    }

    /// Checks whether `action` is permitted on the `type_name` subject type.
    ///
    /// Mirrors class-level `can?` checks: conditions and matchers are not
    /// evaluated, the first relevant rule behavior decides.
    #[must_use]
    pub fn can_check_type(&self, action: &str, type_name: &str) -> bool {
        self.evaluate(action, SubjectRef::Type(type_name), None)
    }

    /// Checks whether `action` is permitted on either subject form.
    #[must_use]
    pub fn can_check_subject(&self, action: &str, subject: SubjectRef<'_>) -> bool {
        self.evaluate(action, subject, None)
    }

    /// Checks whether `action` is permitted on `instance` for `attribute`.
    #[must_use]
    pub fn can_check_attribute(
        &self,
        action: &str,
        instance: &dyn SubjectInstance,
        attribute: &str,
    ) -> bool {
        self.evaluate(action, SubjectRef::Instance(instance), Some(attribute))
    }

    /// Inverse of [`Ability::can_check`], mirroring `cannot?`.
    #[must_use]
    pub fn cannot_check(&self, action: &str, instance: &dyn SubjectInstance) -> bool {
        !self.can_check(action, instance)
    }

    /// Inverse of [`Ability::can_check_type`], mirroring class-level `cannot?`.
    #[must_use]
    pub fn cannot_check_type(&self, action: &str, type_name: &str) -> bool {
        !self.can_check_type(action, type_name)
    }

    /// Inverse of [`Ability::can_check_subject`].
    #[must_use]
    pub fn cannot_check_subject(&self, action: &str, subject: SubjectRef<'_>) -> bool {
        !self.can_check_subject(action, subject)
    }

    /// Inverse of [`Ability::can_check_attribute`].
    #[must_use]
    pub fn cannot_check_attribute(
        &self,
        action: &str,
        instance: &dyn SubjectInstance,
        attribute: &str,
    ) -> bool {
        !self.can_check_attribute(action, instance, attribute)
    }

    /// Checks permission on `instance`, returning [`CanCanError::AccessDenied`] on failure.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::AccessDenied`] when the check fails.
    pub fn authorize(
        &self,
        action: &str,
        instance: &dyn SubjectInstance,
    ) -> Result<(), CanCanError> {
        self.authorize_subject(action, SubjectRef::Instance(instance))
    }

    /// Checks permission on `type_name`, returning [`CanCanError::AccessDenied`] on failure.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::AccessDenied`] when the check fails.
    pub fn authorize_type(&self, action: &str, type_name: &str) -> Result<(), CanCanError> {
        self.authorize_subject(action, SubjectRef::Type(type_name))
    }

    /// Checks permission on either subject form.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::AccessDenied`] when the check fails.
    pub fn authorize_subject(
        &self,
        action: &str,
        subject: SubjectRef<'_>,
    ) -> Result<(), CanCanError> {
        if self.evaluate(action, subject, None) {
            return Ok(());
        }
        let subject_type = match subject {
            SubjectRef::Type(name) => name.to_owned(),
            SubjectRef::Instance(instance) => instance.subject_type().to_owned(),
        };
        Err(CanCanError::AccessDenied {
            action: action.to_owned(),
            subject: subject_type.clone(),
            message: Some(self.unauthorized_message(action, &subject_type)),
        })
    }

    /// [`Ability::authorize_subject`] with an explicit denial message,
    /// mirroring the gem `authorize!(*args, message:)`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::AccessDenied`] carrying `message` verbatim when
    /// the check fails.
    pub fn authorize_message(
        &self,
        action: &str,
        subject: SubjectRef<'_>,
        message: &str,
    ) -> Result<(), CanCanError> {
        if self.evaluate(action, subject, None) {
            return Ok(());
        }
        let subject_type = match subject {
            SubjectRef::Type(name) => name.to_owned(),
            SubjectRef::Instance(instance) => instance.subject_type().to_owned(),
        };
        Err(CanCanError::AccessDenied {
            action: action.to_owned(),
            subject: subject_type,
            message: Some(message.to_owned()),
        })
    }

    /// Checks whether `action` is permitted on at least one subject,
    /// mirroring the gem `can?(action, any: [...])` form.
    #[must_use]
    pub fn can_check_any_of(&self, action: &str, subjects: &[&dyn SubjectInstance]) -> bool {
        subjects
            .iter()
            .any(|subject| self.can_check(action, *subject))
    }

    /// Reverse lookup of aliases expanding to `action`, mirroring the gem
    /// `aliases_for_action` used by the i18n resolver.
    #[must_use]
    pub fn aliases_for_action(&self, action: &str) -> Vec<String> {
        self.actions.aliases_for(action)
    }

    /// Merges every rule and alias from `other` into this ability.
    pub fn merge(&mut self, other: &Ability) -> &mut Self {
        self.rules.extend(other.rules.iter().cloned());
        for target in other.action_targets() {
            let mapped = other.actions.expand(&target);
            let extra: Vec<String> = mapped
                .into_iter()
                .filter(|action| action != &target)
                .collect();
            if !extra.is_empty() {
                self.actions.alias_action(extra, target);
            }
        }
        self
    }

    /// Reports which actions on which subjects carry attribute lists.
    #[must_use]
    pub fn permissions(&self) -> AbilityPermissions {
        let mut allowed: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
        let mut denied: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
        for rule in self.rules.iter().rev() {
            let bucket = if rule.allows() {
                &mut allowed
            } else {
                &mut denied
            };
            let actions = if rule.actions().is_empty() {
                vec!["all".to_owned()]
            } else {
                rule.actions().to_vec()
            };
            let subjects = if rule.subjects().is_empty() {
                vec!["all".to_owned()]
            } else {
                rule.subjects().to_vec()
            };
            for action in actions {
                for subject in &subjects {
                    bucket
                        .entry(action.clone())
                        .or_default()
                        .entry(subject.clone())
                        .or_insert_with(|| rule.attributes().to_vec());
                }
            }
        }
        AbilityPermissions { allowed, denied }
    }

    /// Merges scalar condition values for building new instances.
    ///
    /// Mirrors `attributes_for`: only plain equalities are collected.
    #[must_use]
    pub fn attributes_for(
        &self,
        action: &str,
        subject: &dyn SubjectInstance,
    ) -> HashMap<String, DbValue> {
        let mut attributes = HashMap::new();
        for rule in self.relevant_rules(action, subject.subject_type()) {
            if rule.allows() && rule.matches_instance(subject) {
                attributes.extend(rule.condition().scalar_attributes());
            }
        }
        attributes
    }

    /// Lists attributes permitted for `action` on `subject_type`.
    ///
    /// Mirrors `permitted_attributes`: allow rules add names, deny rules
    /// remove them.
    #[must_use]
    pub fn permitted_attributes(&self, action: &str, subject_type: &str) -> Vec<String> {
        let mut permitted: HashSet<String> = HashSet::new();
        for rule in self.relevant_rules(action, subject_type).into_iter().rev() {
            if rule.attributes().is_empty() {
                continue;
            }
            if rule.allows() {
                permitted.extend(rule.attributes().iter().cloned());
            } else {
                for attribute in rule.attributes() {
                    permitted.remove(attribute);
                }
            }
        }
        let mut sorted: Vec<String> = permitted.into_iter().collect();
        sorted.sort();
        sorted
    }

    /// Whether any relevant rule carries a raw SQL fragment.
    ///
    /// Mirrors `has_raw_sql?`.
    #[must_use]
    pub fn has_raw_sql(&self, action: &str, subject_type: &str) -> bool {
        self.relevant_rules(action, subject_type)
            .iter()
            .any(|rule| rule.has_raw_sql())
    }

    /// Whether any relevant rule carries a block matcher.
    #[must_use]
    pub fn has_matcher(&self, action: &str, subject_type: &str) -> bool {
        self.relevant_rules(action, subject_type)
            .iter()
            .any(|rule| rule.has_matcher())
    }

    /// Returns relevant rules usable for database queries.
    ///
    /// Mirrors `relevant_rules_for_query`: `cannot` rules carrying attribute
    /// lists are rejected (attributes only filter in-memory checks), and a
    /// relevant block-matcher rule fails the whole query.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::BlockInQuery`] when a relevant rule carries a
    /// block matcher, mirroring `relevant_rules_for_query`.
    pub fn rules_for_query(
        &self,
        action: &str,
        subject_type: &str,
    ) -> Result<Vec<Rule>, CanCanError> {
        let mut relevant: Vec<Rule> = Vec::new();
        for rule in self.relevant_rules(action, subject_type) {
            if !rule.allows() && !rule.attributes().is_empty() {
                continue;
            }
            if rule.has_matcher() {
                return Err(CanCanError::BlockInQuery);
            }
            relevant.push(rule.clone());
        }
        Ok(relevant)
    }

    /// Resolves the denial message for `action` on `subject_type`.
    #[must_use]
    pub fn unauthorized_message(&self, action: &str, subject_type: &str) -> String {
        if let Some(resolver) = &self.message_resolver {
            for candidate_action in self.actions.expand(action) {
                if let Some(message) = resolver(&candidate_action, subject_type) {
                    return message;
                }
            }
        }
        default_message(action, subject_type)
    }

    /// Every rule in definition order.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    fn evaluate(&self, action: &str, subject: SubjectRef<'_>, attribute: Option<&str>) -> bool {
        let (subject_type, instance) = match subject {
            SubjectRef::Type(name) => (name, None),
            SubjectRef::Instance(found) => (found.subject_type(), Some(found)),
        };
        for rule in &self.relevant_rules(action, subject_type) {
            debug_assert!(rule.is_relevant(&self.actions, action, subject_type));
            let instance_matches = match instance {
                Some(found) => rule.matches_instance(found),
                None => rule.allows(),
            };
            if instance_matches && rule.matches_attribute(attribute) {
                return rule.allows();
            }
        }
        false
    }

    fn relevant_rules(&self, action: &str, subject_type: &str) -> Vec<&Rule> {
        self.rules
            .iter()
            .filter(|rule| rule.is_relevant(&self.actions, action, subject_type))
            .rev()
            .collect()
    }

    fn action_targets(&self) -> Vec<String> {
        let mut targets: Vec<String> = Vec::new();
        for rule in &self.rules {
            for action in rule.actions() {
                if !targets.contains(action) {
                    targets.push(action.clone());
                }
            }
        }
        targets
    }
}

impl Default for Ability {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Ability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Ability")
            .field("rules", &self.rules)
            .field("actions", &self.actions)
            .field("has_message_resolver", &self.message_resolver.is_some())
            .finish()
    }
}

/// Attribute lists exposed by an ability, split by rule behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AbilityPermissions {
    /// `action -> subject -> attributes` for `allow` rules.
    pub allowed: HashMap<String, HashMap<String, Vec<String>>>,
    /// `action -> subject -> attributes` for `deny` rules.
    pub denied: HashMap<String, HashMap<String, Vec<String>>>,
}
