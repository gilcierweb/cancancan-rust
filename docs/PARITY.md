# Gem Parity Audit

Audit of every public function of the Ruby `CanCan::Ability` (gem 3.6.0) against
`cancancan-core`, plus the directives that interact with controllers. Items are
marked as:

- ✅ implemented (zeroed in Rust)
- ⚠️ adapted (semantics preserved via different Rust constructs)
- ❌ deferred (not yet ported — tracked below)

## `CanCan::Ability` — method by method

| Gem method | Rust implementation | Status |
|---|---|---|
| `can?` (instance) | `Ability::can_check` | ✅ |
| `can?` (class/type) | `Ability::can_check_type` | ✅ |
| `can?(action, subject, attribute)` | `Ability::can_check_attribute` | ✅ |
| `cannot?` | `Ability::cannot_check` + `_type` | ✅ |
| `can action, subject` (rule definition) | `Ability::can`, `can_where`, `can_matching`, `can_attributes` | ✅ |
| `cannot` (rule definition) | `Ability::cannot`, `cannot_where`, `cannot_matching`, `cannot_attributes` | ✅ |
| `authorize!` | `Ability::authorize`, `authorize_type`, `authorize_subject` | ✅ |
| `authorize!(…, message:)` | `Ability::authorize_message` | ✅ |
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
| `check_authorization` (ddos guard 500 if no authorize) | `cancancan_axum::check_authorization` / `cancancan_actix::check_authorization` middleware | ✅ |
| `skip_authorization_check` | `CurrentAbility::skip_authorization_check()` | ✅ |
| `load_and_authorize_resource` / `load_resource` / `authorize_resource` (load via DB by id/collection) | not ported (would require framework+ORM bridging layer) | ❌ deferred |

## Deferred (not ported — tracked)

- **`load_and_authorize_resource` family**: Rails-side controller resource loader that fetches records through ActiveRecord. Needs the ORM adapters + web framework integration to be combined intentionally; left as future work (could be a `cancancan-web` helper crate that builds a `CurrentAbility`-style extractor accepting a `loader` closure).
- **Extendable `authorize` exposure to views** — not appliable in Rust (no helpers/view layer automatically).
- **Generator CLI** (`rails g cancan:ability`): `cancancan-cli` scaffold exists (crate-level, not yet registered in releases).

## Known divergences (documented in code)

1. Class-level checks (`can_check_type`) do **not** evaluate conditions/matchers — identical to the gem (`can?` on a Class ignores conditions).
2. Adapter `Nested` conditions are **rejected with `WrongAssociation`** in Diesel/SeaORM/SQLx adapters (join inference not implemented); MongoDB supports them natively via dot-notation.
3. **UUID**: SQLx binds UUID as validated text; Diesel typed path binds natively on Postgres (`sql_types::Uuid`) and validated text on SQLite; SeaORM binds native `sea-query::Value::Uuid`; Mongo binds native BSON Binary (subtype 4). Integers are always bound (i16/i32/i64) with overflow checks.
4. **Null semantics in MongoDB are strict**: `IS NULL` → `$type: "null"` (present null only), unlike the loose `{ field: null }` that would also match missing fields.
5. `CannotAttributes` rules are **rejected** by adapters' `rules_for_query` — same as the gem's `relevant_rules_for_query`.
