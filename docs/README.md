# CanCanCan-Rust - Developer Guide

A declarative authorization library for Rust, ported from the Ruby
[CanCanCan](https://github.com/CanCanCommunity/cancancan) gem. Each page
adapts a chapter of the gem's official documentation to the Rust API.

## Table of contents

### Getting started
- [Define and check abilities](./define_check_abilities.md) - `can` / `cannot` / `can_check`
- [Rules](./rules.md) - the rule model: action, subject, condition, attributes
- [Conditions](./conditions.md) - the `Condition` / `DbValue` reference
- [Hash of conditions](./hash_of_conditions.md) - restricting rules to records

### Going deeper with definitions
- [Combine abilities](./combine_abilities.md) - precedence, OR semantics, `merge`
- [Cannot](./cannot.md) - denials and how they interact with grants
- [Split the ability definitions](./split_ability.md) - per-module organization
- [Define abilities with matchers](./define_abilities_with_matchers.md) - closure rules
- [Subjects](./subjects.md) - `SubjectInstance`, ad-hoc and map-backed subjects
- [Accessible attributes](./accessible_attributes.md) - attribute-level rules and checks
- [Permitted attributes](./permitted_attributes.md) - `permitted_attributes` + `attributes_for` for input filtering
- [Aliases](./aliases.md) - action aliasing and the `manage` / `all` wildcards
- [Define abilities - best practices](./define_abilities_best_practices.md) - give permissions, don't take them
- [Changing defaults](./changing_defaults.md) - default actions, aliases and resolvers
- [Rules compression](./rules_compression.md) - automatic rule-list optimization
- [Role-based authorization](./role_based_authorization.md) - roles in constants, bitmasks, inheritance
- [Storing abilities in the database](./abilities_in_database.md) - DB-backed rule sets
- [Accessing request data](./accessing_request_data.md) - IPs, session, tenant headers

### Authorizing requests
- [Checking abilities](./checking.md) - `can_check` / `authorize` / `cannot_check`
- [Check abilities - common mistakes](./check_abilities_mistakes.md) - type vs instance checks
- [Handling access denied](./handling_access_denied.md) - `CanCanError`, 403/404 policy
- [Internationalization](./internationalization.md) - translated denial messages
- [Web integration](./web-integration.md) - axum + actix extractors
- [Error handling](./error-handling.md) - the full `CanCanError` surface
- [Debugging](./debugging.md) - inspecting rules and check outcomes
- [Testing](./testing.md) - unit, query and request layers

### Query adapters
- [Query adapters - overview](./query-adapters.md) - `accessible_by` for Diesel / SeaORM / SQLx / MongoDB
- [Model adapters](./model_adapter.md) - writing your own backend adapter
- [SQL strategies](./sql_strategies.md) - current SQL-generation behavior

### Rails-specific chapters (notes)
- [FriendlyId / slugs](./friendly_id.md) - not ported; you own record loading
- [Inherited Resources / nested resources](./inherited_resources.md) - not ported; no Rails controllers

## Reference documents

- [PARITY.md](./PARITY.md) - gem-to-port feature matrix
- [INTEGRATION.md](./INTEGRATION.md) - `rolify-rust` + authentication walkthrough

## Installation

```toml
[dependencies]
cancancan-core = "*"
cancancan-axum = "*"     # or cancancan-actix
cancancan-diesel = "*"   # or cancancan-sqlx, cancancan-seaorm, cancancan-mongo
```

Run the test suite (needs Docker for the MongoDB e2e tests):

```bash
cargo test --workspace
```
