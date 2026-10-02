use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::actions::Actions;
use crate::condition::{DbValue, SubjectInstance};
use crate::error::CanCanError;
use crate::messages::default_message;
use crate::rule::{BlockMatcher, Rule};
use crate::Condition;

/// Subject an ability check runs against.
#[derive(Clone, Copy)]
pub enum SubjectRef<'subject> {
    /// Check against a subject type name (e.g. `"Post"`), mirroring class checks.
    Type(&'subject str),
    /// Check against a concrete instance, mirroring instance checks.
    Instance(&'subject dyn SubjectInstance),
}

impl<'subject> From<&'subject str> for SubjectRef<'subject> {
    fn from(type_name: &'subject str) -> Self {
        Self::Type(type_name)
    }
}

impl<'subject> From<&'subject dyn SubjectInstance> for SubjectRef<'subject> {
    fn from(instance: &'subject dyn SubjectInstance) -> Self {
        Self::Instance(instance)
    }
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
pub type MessageResolver = Rc<dyn Fn(&str, &str) -> Option<String>>;

/// Defines and checks authorization rules.
///
/// Mirrors `CanCan::Ability`: rules are declared with `allow`/`deny`
/// (the gem `can`/`cannot`), checked with [`Ability::can`],
/// [`Ability::cannot`] and [`Ability::authorize`]. The last matching rule wins.
///
/// # Example
///
/// ```rust
/// use cancancan_core::{Ability, Condition, SubjectInstance, DbValue};
///
/// struct Post { user_id: i64 }
///
/// impl SubjectInstance for Post {
///     fn subject_type(&self) -> &str { "Post" }
///     fn attribute(&self, name: &str) -> Option<DbValue> {
///         if name == "user_id" { Some(DbValue::Int(self.user_id)) } else { None }
///     }
/// }
///
/// let mut ability = Ability::new();
/// ability.allow_where("read", "Post", Condition::Eq {
///     field: "user_id".to_owned(),
///     value: DbValue::Int(1),
/// }).unwrap();
///
/// assert!(ability.can("read", &Post { user_id: 1 }));
/// assert!(!ability.can("read", &Post { user_id: 2 }));
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
    pub fn alias_action<I, S>(&mut self, actions: I, target: S) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.actions.alias_action(actions, target);
        self
    }

    /// Removes every action alias, including the defaults.
    pub fn clear_aliases(&mut self) -> &mut Self {
        self.actions.clear();
        self
    }

    /// Overrides how denial messages are resolved.
    pub fn set_message_resolver(&mut self, resolver: MessageResolver) -> &mut Self {
        self.message_resolver = Some(resolver);
        self
    }

    /// Declares an `allow` rule for `action` on `subject_type`.
    ///
    /// Pass `None` for either argument to match everything, mirroring
    /// `can` without arguments.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn allow(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::allow(action.map(str::to_owned), subject_type.map(str::to_owned))?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `deny` rule for `action` on `subject_type`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn deny(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::deny(action.map(str::to_owned), subject_type.map(str::to_owned))?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares an `allow` rule guarded by a declarative `condition`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn allow_where(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        condition: Condition,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::allow_where(
            action.map(str::to_owned),
            subject_type.map(str::to_owned),
            condition,
        )?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `deny` rule guarded by a declarative `condition`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn deny_where(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        condition: Condition,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::deny_where(
            action.map(str::to_owned),
            subject_type.map(str::to_owned),
            condition,
        )?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares an `allow` rule evaluated by `matcher` at check time.
    ///
    /// Matcher rules only run in memory; adapters reject them for queries.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn allow_matching(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        matcher: BlockMatcher,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::allow_matching(
            action.map(str::to_owned),
            subject_type.map(str::to_owned),
            matcher,
        )?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares a `deny` rule evaluated by `matcher` at check time.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn deny_matching(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        matcher: BlockMatcher,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::deny_matching(
            action.map(str::to_owned),
            subject_type.map(str::to_owned),
            matcher,
        )?;
        self.rules.push(rule);
        Ok(self)
    }

    /// Declares an `allow` rule exposing `attributes` for parameter filtering.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `action` is `Some`
    /// while `subject_type` is `None`.
    pub fn allow_attributes(
        &mut self,
        action: Option<&str>,
        subject_type: Option<&str>,
        attributes: Vec<String>,
    ) -> Result<&mut Self, CanCanError> {
        let rule = Rule::allow(action.map(str::to_owned), subject_type.map(str::to_owned))?
            .with_attributes(attributes);
        self.rules.push(rule);
        Ok(self)
    }

    /// Checks whether `action` is permitted on `subject`.
    #[must_use]
    pub fn can<'subject>(
        &self,
        action: &str,
        subject: impl Into<SubjectRef<'subject>>,
    ) -> bool {
        self.check(action, subject.into(), None)
    }

    /// Checks whether `action` is permitted on `subject` for `attribute`.
    #[must_use]
    pub fn can_on_attribute<'subject>(
        &self,
        action: &str,
        subject: impl Into<SubjectRef<'subject>>,
        attribute: &str,
    ) -> bool {
        self.check(action, subject.into(), Some(attribute))
    }

    /// Inverse of [`Ability::can`].
    #[must_use]
    pub fn cannot<'subject>(
        &self,
        action: &str,
        subject: impl Into<SubjectRef<'subject>>,
    ) -> bool {
        !self.can(action, subject)
    }

    /// Checks permission, returning [`CanCanError::AccessDenied`] on failure.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::AccessDenied`] when the check fails.
    pub fn authorize<'subject>(
        &self,
        action: &str,
        subject: impl Into<SubjectRef<'subject>>,
    ) -> Result<(), CanCanError> {
        let reference = subject.into();
        if self.check(action, reference, None) {
            return Ok(());
        }
        let subject_type = match reference {
            SubjectRef::Type(name) => name.to_owned(),
            SubjectRef::Instance(instance) => instance.subject_type().to_owned(),
        };
        Err(CanCanError::AccessDenied {
            action: action.to_owned(),
            subject: subject_type.clone(),
            message: Some(self.unauthorized_message(action, &subject_type)),
        })
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
        AbilityPermissions {
            allowed,
            denied,
        }
    }

    /// Merges scalar condition values for building new instances.
    ///
    /// Mirrors `attributes_for`: only plain equalities are collected.
    #[must_use]
    pub fn attributes_for(&self, action: &str, subject: &dyn SubjectInstance) -> HashMap<String, DbValue> {
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

    /// Whether any relevant rule carries a block matcher.
    #[must_use]
    pub fn has_matcher(&self, action: &str, subject_type: &str) -> bool {
        self.relevant_rules(action, subject_type)
            .iter()
            .any(|rule| rule.has_matcher())
    }

    /// Returns relevant rules usable for database queries.
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
        let relevant = self.relevant_rules(action, subject_type);
        if relevant.iter().any(|rule| rule.has_matcher()) {
            return Err(CanCanError::BlockInQuery);
        }
        Ok(relevant.into_iter().cloned().collect())
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

    fn check(&self, action: &str, subject: SubjectRef<'_>, attribute: Option<&str>) -> bool {
        let expanded = self.actions.expand(action);
        let (subject_type, instance) = match subject {
            SubjectRef::Type(name) => (name, None),
            SubjectRef::Instance(found) => (found.subject_type(), Some(found)),
        };
        for rule in self.relevant_rules(action, subject_type).iter() {
            debug_assert!(rule.is_relevant(&expanded, subject_type));
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
        let expanded = self.actions.expand(action);
        self.rules
            .iter()
            .filter(|rule| rule.is_relevant(&expanded, subject_type))
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

/// Attribute lists exposed by an ability, split by rule behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AbilityPermissions {
    /// `action -> subject -> attributes` for `allow` rules.
    pub allowed: HashMap<String, HashMap<String, Vec<String>>>,
    /// `action -> subject -> attributes` for `deny` rules.
    pub denied: HashMap<String, HashMap<String, Vec<String>>>,
}
