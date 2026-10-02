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
//! | `merge`                 | [`Ability::merge`]                  |
//! | `permissions`           | [`Ability::permissions`]            |
//! | `attributes_for`        | [`Ability::attributes_for`]         |
//! | `permitted_attributes`  | [`Ability::permitted_attributes`]   |
//! | hash conditions         | [`Condition`]                       |
//! | `:manage` / `:all`      | `"manage"` / `"all"` strings        |

mod ability;
mod actions;
mod compressor;
mod condition;
mod error;
mod messages;
mod rule;

pub use ability::{Ability, AbilityPermissions, MessageResolver, SubjectRef};
pub use actions::Actions;
pub use compressor::compress;
pub use condition::{Condition, DbValue, SubjectInstance};
pub use error::CanCanError;
pub use messages::default_message;
pub use rule::{BlockMatcher, Rule};
