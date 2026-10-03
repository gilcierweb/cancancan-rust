//! Authorization library inspired by the Ruby `CanCanCan` gem.
//!
//! [`Ability`] holds ordered `allow`/`deny` rules (the gem `can`/`cannot`).
//! Checks run against a subject type name or a concrete instance implementing
//! [`SubjectInstance`]. The last matching rule wins.
//!
//! Backend query support (`accessible_by`) lives in adapter crates
//! (`cancancan-diesel`, `cancancan-seaorm`, `cancancan-sqlx`,
//! `cancancan-mongo`), which translate [`Condition`] trees into queries.
//!
//! Gem to Rust API mapping:
//!
//! | Ruby gem                | This crate                          |
//! |-------------------------|-------------------------------------|
//! | `can` / `cannot`        | [`Ability::can`] / [`Ability::cannot`] (+ `_where`, `_matching`, `_attributes`) |
//! | `can?` / `cannot?`      | [`Ability::can_check`] / [`Ability::cannot_check`] ([`Ability::can_check_type`] for classes) |
//! | `authorize!`            | [`Ability::authorize`] / [`Ability::authorize_type`] |
//! | `alias_action`          | [`Ability::alias_action`]           |
//! | `aliased_actions`       | [`Ability::aliased_actions`]        |
//! | `clear_aliased_actions` | [`Ability::clear_aliased_actions`]  |
//! | `merge`                 | [`Ability::merge`]                  |
//! | `permissions`           | [`Ability::permissions`]            |
//! | `attributes_for`        | [`Ability::attributes_for`]         |
//! | `permitted_attributes`  | [`Ability::permitted_attributes`]   |
//! | `has_block?`            | [`Ability::has_matcher`]            |
//! | `has_raw_sql?`          | [`Ability::has_raw_sql`]            |
//! | hash conditions         | [`Condition`]                       |
//! | `:manage` / `:all`      | `"manage"` / `"all"` strings        |
//!
//! Translation rules where Rust cannot spell the gem: `?` becomes a `_check`
//! suffix, `!` is dropped (`authorize!` pairs with the `?` operator), and
//! `Error` suffixes are dropped from error variants (the enum is the error).

mod ability;
mod actions;
mod compressor;
mod condition;
mod config;
mod error;
mod messages;
mod rule;
mod validation;

pub use ability::{Ability, AbilityPermissions, MessageResolver, SubjectRef};
pub use actions::Actions;
pub use compressor::compress;
pub use condition::{Condition, DbValue, MapSubject, SubjectInstance};
pub use config::{rules_compressor_enabled, set_rules_compressor_enabled};
pub use error::CanCanError;
pub use messages::default_message;
pub use rule::{BlockMatcher, Rule};
pub use validation::is_identifier;
