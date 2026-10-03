# Gem Parity Audit

Audit of every public function of the Ruby `CanCan::Ability` (gem 3.6.0) against
`cancancan-core`, plus the directives that interact with controllers. Items are
marked as:

- ✅ implemented (zeroed in Rust)
- ⚠️ adapted (semantics preserved via different Rust constructs)
- ❌ deferred (not yet ported - tracked below)

## `CanCan::Ability` - method by method

| Gem method | Rust implementation | Status |
|---|---|---|
| `can?` (instance) | `Ability::can_check` | ✅ |
| `can?` (class/type) | `Ability::can_check_type` | ✅ |
| `can?(action, subject, attribute)` | `Ability::can_check_attribute` | ✅ |
| `cannot?` | `Ability::cannot_check` + `_type` | ✅ |
| `can action, subject` (rule definition) | `Ability::can`, `can_where`, `can_matching`, `can_attributes` | ✅ |
| `can action, subject, attributes, conditions` (combined) | `Ability::can_attributes_where`, `cannot_attributes_where` | ✅ |
| `cannot` (rule definition) | `Ability::cannot`, `cannot_where`, `cannot_matching`, `cannot_attributes` | ✅ |
| `authorize!` | `Ability::authorize`, `authorize_type`, `authorize_subject` | ✅ |
| `authorize!(..., message:)` | `Ability::authorize_message` | ✅ |
| `alias_action` (+ `validate_target` guard) | `Ability::alias_action` -> `Result`, `CanCanError::InvalidAliasTarget` | ✅ |
| `aliased_actions` | `Ability::aliased_actions` | ✅ |
| `clear_aliased_actions` | `Ability::clear_aliased_actions` | ✅ |
| `aliases_for_action` | `Ability::aliases_for_action` | ✅ |
| `merge` | `Ability::merge` (rules + aliases) | ✅ |
| `permissions` | `Ability::permissions` | ✅ |
| `attributes_for` | `Ability::attributes_for` | ✅ |
| `permitted_attributes` (strong params) | `Ability::permitted_attributes` | ✅ (registration by attribute set instead of Rails params) |
| `has_block?` | `Ability::has_matcher` | ✅ |
| `has_raw_sql?` | `Ability::has_raw_sql` | ✅ |
| `unauthorized_message` (i18n resolver) | `Ability::unauthorized_message` + `set_message_resolver` | ✅ (plain resolver, no i18n backend required) |
| `model_adapter(reflection)` | per-ORM adapters (`cancancan-diesel`, `-seaorm`, `-sqlx`, `-mongo`) | ✅ |
| `relevant_rules_for_query` | `Ability::rules_for_query` (rejects blocks, cannot+attributes) | ✅ |
| `rules` (internal) | `Ability::rules` | ✅ |
| `compress` (rules compressor) | `cancancan_core::compress` + `rules_compressor_enabled` toggle | ✅ |
| `Condition` hash (eq/ne/in/range/is_null/nested/raw sql) | `Condition` enum (Eq, Ne, In, Range, IsNull, And, Or, Not, Nested, RawSql) | ✅ |
| `:manage`/`:all` wildcard | `"manage"`/`"all"` strings | ✅ |
| `can?` with `any: [...]` subjects | `Ability::can_check_any_of` | ✅ |
| block matchers | `can_matching`/`cannot_matching` (`Arc<dyn Fn>`) | ✅ (thread-safe) |

## ControllerAdditions (Rails DSL → web framework integrations)

| Gem directive | Rust (~axum/actix crates) | Status |
|---|---|---|
| `can?`/`cannot?` in controllers/views | `CurrentAbility::can_check`/`cannot_check` extractors | ✅ |
| `authorize!` in controller | `CurrentAbility::authorize`/`authorize_type` (403/500 mapping) | ✅ |
| `load_resource` (load record only, 404/500) | `CurrentAbility::load_resource` | ✅ |
| `authorize_resource` (authorize already-loaded) | `CurrentAbility::authorize_resource` | ✅ |
| `load_and_authorize_resource` | `CurrentAbility::load_and_authorize` | ✅ |
| `check_authorization` (500 if no authorize on success) | `check_authorization` middleware (axe+actix) | ✅ |
| `skip_authorization_check` | `CurrentAbility::skip_authorization_check()` | ✅ |
| Rails controller options (`:through`, `:shallow`, `:singleton`, `:parent`, `:class`, `:instance_name`) | encode Rails conventions with no Rust equivalent; loaders own the DB lookup closure | ❌ by design |

## Deferred (not ported - by design)

- **Rails controller-macro options** (`:through`, `:shallow`, `:singleton`,
  `:parent`, `:class`, `:instance_name`): those encode Rails conventions with no
  direct Rust equivalent. Handlers call `load_and_authorize` with a loader
  closure that owns the DB lookup instead.
- **Views**: Rails helpers (`can?`/`cannot?` in templates) map to calling
  `can_check` wherever you hold the ability; no separate integration needed.
- **Generator CLI** (`rails g cancan:ability`): `cancancan-cli scaffold` exists.

## Known divergences (documented in code)

1. Class-level checks (`can_check_type`) ignore conditions and matchers: a
   catch-all `cannot` still denies (last-match-wins), a conditional/matcher
   rule matches by its base behavior - mirroring the gem's
   `matches_non_block_conditions`/`matches_block_conditions` class branch.
2. Adapter `Nested` conditions are **rejected with `WrongAssociation`** in Diesel/SeaORM/SQLx adapters (join inference not implemented); MongoDB supports them natively via dot-notation.
3. **UUID**: SQLx binds UUID as validated text; Diesel typed path binds natively on Postgres (`sql_types::Uuid`) and validated text on SQLite; SeaORM binds native `sea-query::Value::Uuid`; Mongo binds native BSON Binary (subtype 4). Integers are always bound (i16/i32/i64) with overflow checks.
4. **Null semantics in MongoDB are strict**: `IS NULL` → `$type: "null"` (present null only), unlike the loose `{ field: null }` that would also match missing fields.
5. `CannotAttributes` rules are **rejected** by adapters' `rules_for_query` - same as the gem's `relevant_rules_for_query`.
6. Raw-SQL conditions are skipped (not raised) by in-memory `can_check` - the gem raises; a boolean API cannot raise, so the rule is treated as non-matching (`has_raw_sql` exposes this).
7. `unauthorized_message` resolves through a closure with the gem's key chain (`action`/`aliases`/`manage` × `subject`/`all`); no i18n backend is bundled.

