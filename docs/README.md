# CanCanCan-Rust — Developer Guide

A declarative authorization library for Rust, fully ported from the Ruby CanCanCan gem.
Each page in this section covers a different section of the official guide.

## Table of contents

### Defining abilities
- [Define and check abilities](define-check.md) — what `can` / `cannot` really are
- [Defining abilities — best practices](define_abilities_best_practices.md) — rules with progressive grants
- [Defining abilities with matchers (bridge for closures)](matchers.md) — runtime-only rules
- [Rule API (low-level)](rules.md) — `Ability`, `Rule`, `Condition`, aliases
- [Accessible attributes](permitted_attributes.md) — `attributes_for` + `rules_for_query`

### Authorizing in a framework
- [Web integration (Axum + actix)](web-integration.md) — extractors + `check_authorization`
- [Error handling](error-handling.md) — status codes and HTTP errors

### Query adapters
- [Query adapters — overview](query-adapters.md)
- [Diesel](adapters-diesel.md)
- [SeaORM](adapters-seaorm.md)
- [SQLx](adapters-sqlx.md)
- [MongoDB](adapters-mongo.md)

### Style, integration, loading
- [Action aliases](aliases.md) — read → index/show; create → new; update → edit
- [Debugging your Ability](debugging.md)
- [Custom denial messages](custom_messages.md)
- [Records stored in the database and custom authorities](abilities_in_database.md)

The parity matrix and the ability-to-framework-export are in [PARITY.md](PARITY.md).

## Installation

Add the specific adapter you need (features):

```toml
[dependencies]
cancancan-core = "*"
cancancan-axum = "*"         # or cancancan-actix
cancancan-diesel = "*"       # or cancancan-sqlx, cancancan-seaorm, cancancan-mongo
```

The tests and e2e docker coverage exists with Docker for mongo. Check `cargo test --workspace`.
