use std::rc::Rc;

use crate::actions::Actions;
use crate::condition::{Condition, SubjectInstance};
use crate::error::CanCanError;

/// Block matcher evaluated against an instance at check time.
///
/// Rules carrying a matcher cannot be translated into database queries,
/// mirroring the `only_block?` guard of the Ruby gem.
pub type BlockMatcher = Rc<dyn Fn(&dyn SubjectInstance) -> bool>;

/// Single `allow`/`deny` declaration, mirroring `CanCan::Rule`.
///
/// The last matching rule wins when an ability is checked.
#[derive(Clone)]
pub struct Rule {
    allow: bool,
    match_all_actions: bool,
    actions: Vec<String>,
    match_all_subjects: bool,
    subjects: Vec<String>,
    attributes: Vec<String>,
    condition: Condition,
    matcher: Option<BlockMatcher>,
}

impl Rule {
    fn build(
        allow: bool,
        action: Option<String>,
        subject: Option<String>,
        condition: Condition,
        matcher: Option<BlockMatcher>,
    ) -> Result<Self, CanCanError> {
        if action.is_some() && subject.is_none() {
            return Err(CanCanError::ActionWithoutSubject);
        }
        if matcher.is_some() && !matches!(condition, Condition::All) {
            return Err(CanCanError::BlockAndConditions);
        }
        Ok(Self {
            allow,
            match_all_actions: action.is_none(),
            actions: action.into_iter().collect(),
            match_all_subjects: subject.is_none(),
            subjects: subject.into_iter().collect(),
            attributes: Vec::new(),
            condition,
            matcher,
        })
    }

    /// Defines a `can` rule for `action` on `subject`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `subject` is `None`
    /// while `action` is `Some`.
    pub fn can(
        action: Option<impl Into<String>>,
        subject: Option<impl Into<String>>,
    ) -> Result<Self, CanCanError> {
        Self::build(
            true,
            action.map(Into::into),
            subject.map(Into::into),
            Condition::All,
            None,
        )
    }

    /// Defines a `cannot` rule for `action` on `subject`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `subject` is `None`
    /// while `action` is `Some`.
    pub fn cannot(
        action: Option<impl Into<String>>,
        subject: Option<impl Into<String>>,
    ) -> Result<Self, CanCanError> {
        Self::build(
            false,
            action.map(Into::into),
            subject.map(Into::into),
            Condition::All,
            None,
        )
    }

    /// Defines a `can` rule guarded by declarative `condition`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `subject` is `None`
    /// while `action` is `Some`.
    pub fn can_where(
        action: Option<impl Into<String>>,
        subject: Option<impl Into<String>>,
        condition: Condition,
    ) -> Result<Self, CanCanError> {
        Self::build(
            true,
            action.map(Into::into),
            subject.map(Into::into),
            condition,
            None,
        )
    }

    /// Defines a `cannot` rule guarded by declarative `condition`.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `subject` is `None`
    /// while `action` is `Some`.
    pub fn cannot_where(
        action: Option<impl Into<String>>,
        subject: Option<impl Into<String>>,
        condition: Condition,
    ) -> Result<Self, CanCanError> {
        Self::build(
            false,
            action.map(Into::into),
            subject.map(Into::into),
            condition,
            None,
        )
    }

    /// Defines a `can` rule evaluated by `matcher` at check time.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `subject` is `None`
    /// while `action` is `Some`.
    pub fn can_matching(
        action: Option<impl Into<String>>,
        subject: Option<impl Into<String>>,
        matcher: BlockMatcher,
    ) -> Result<Self, CanCanError> {
        Self::build(
            true,
            action.map(Into::into),
            subject.map(Into::into),
            Condition::All,
            Some(matcher),
        )
    }

    /// Defines a `cannot` rule evaluated by `matcher` at check time.
    ///
    /// # Errors
    ///
    /// Returns [`CanCanError::ActionWithoutSubject`] when `subject` is `None`
    /// while `action` is `Some`.
    pub fn cannot_matching(
        action: Option<impl Into<String>>,
        subject: Option<impl Into<String>>,
        matcher: BlockMatcher,
    ) -> Result<Self, CanCanError> {
        Self::build(
            false,
            action.map(Into::into),
            subject.map(Into::into),
            Condition::All,
            Some(matcher),
        )
    }

    /// Restricts this rule to the given attribute list for `permitted_attributes`.
    #[must_use]
    pub fn with_attributes(mut self, attributes: Vec<String>) -> Self {
        self.attributes = attributes;
        self
    }

    /// Whether this is an `allow` rule (`true`) or a `deny` rule (`false`).
    #[must_use]
    pub fn allows(&self) -> bool {
        self.allow
    }

    /// Actions this rule applies to; empty when it matches every action.
    #[must_use]
    pub fn actions(&self) -> &[String] {
        &self.actions
    }

    /// Subjects this rule applies to; empty when it matches every subject.
    #[must_use]
    pub fn subjects(&self) -> &[String] {
        &self.subjects
    }

    /// Attribute names this rule exposes for parameter filtering.
    #[must_use]
    pub fn attributes(&self) -> &[String] {
        &self.attributes
    }

    /// Declarative condition attached to this rule.
    #[must_use]
    pub fn condition(&self) -> &Condition {
        &self.condition
    }

    /// Whether this rule carries a block matcher instead of conditions.
    #[must_use]
    pub fn has_matcher(&self) -> bool {
        self.matcher.is_some()
    }

    /// Whether this rule carries a raw SQL fragment, mirroring `only_raw_sql?`.
    #[must_use]
    pub fn has_raw_sql(&self) -> bool {
        self.matcher.is_none() && matches!(self.condition, Condition::RawSql(_))
    }

    /// Whether this rule matches without any condition or matcher.
    #[must_use]
    pub fn is_catch_all(&self) -> bool {
        matches!(self.condition, Condition::All) && self.matcher.is_none()
    }

    /// Whether this is a `deny` rule matching everything, mirroring `cannot_catch_all?`.
    #[must_use]
    pub fn is_deny_catch_all(&self) -> bool {
        !self.allow && self.is_catch_all()
    }

    /// Whether this rule matches every action (`allow` without action).
    #[must_use]
    pub fn matches_all_actions(&self) -> bool {
        self.match_all_actions
    }

    /// Whether this rule covers `action`, expanding aliases from the rule side.
    ///
    /// A rule defined on `read` covers `index` and `show`; `manage` covers all.
    #[must_use]
    pub fn covers_action(&self, actions: &Actions, action: &str) -> bool {
        if self.match_all_actions {
            return true;
        }
        let wanted = action.to_owned();
        self.actions.iter().any(|defined| {
            defined == "manage"
                || defined == "all"
                || defined == &wanted
                || actions.expand(defined).contains(&wanted)
        })
    }

    /// Whether this rule applies to the given subject type name.
    #[must_use]
    pub fn matches_subject(&self, subject_type: &str) -> bool {
        if self.match_all_subjects {
            return true;
        }
        self.subjects
            .iter()
            .any(|subject| subject == "all" || subject == subject_type)
    }

    /// Whether this rule is relevant for `action` on `subject_type`.
    #[must_use]
    pub fn is_relevant(&self, actions: &Actions, action: &str, subject_type: &str) -> bool {
        self.covers_action(actions, action) && self.matches_subject(subject_type)
    }

    /// Whether this rule matches `instance` (conditions plus block matcher).
    #[must_use]
    pub fn matches_instance(&self, instance: &dyn SubjectInstance) -> bool {
        if !self.condition.matches(instance) {
            return false;
        }
        self.matcher.as_ref().is_none_or(|check| check(instance))
    }

    /// Whether this rule matches `attribute` for parameter filtering.
    ///
    /// Mirrors `matches_attributes?`: no attribute list matches everything,
    /// a missing attribute falls back to the rule behavior.
    #[must_use]
    pub fn matches_attribute(&self, attribute: Option<&str>) -> bool {
        if self.attributes.is_empty() {
            return true;
        }
        match attribute {
            None => self.allow,
            Some(name) => self.attributes.iter().any(|item| item == name),
        }
    }
}

impl std::fmt::Debug for Rule {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Rule")
            .field("allow", &self.allow)
            .field("match_all_actions", &self.match_all_actions)
            .field("actions", &self.actions)
            .field("match_all_subjects", &self.match_all_subjects)
            .field("subjects", &self.subjects)
            .field("attributes", &self.attributes)
            .field("condition", &self.condition)
            .field("has_matcher", &self.has_matcher())
            .finish_non_exhaustive()
    }
}
