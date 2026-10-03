# SQL strategies

> **Status: deferred.** The gem lets you pick how `accessible_by` SQL is
> generated (`:left_join` vs `:subquery`). The port currently generates one
> strategy per adapter; a pluggable strategy switch is on the roadmap.

## What the gem's option does

Given a rule that traverses associations:

```ruby
can :read, Article, mentions: { user: { name: u.name } }
```

the gem's default `:left_join` strategy emits `DISTINCT` + joins:

```sql
SELECT DISTINCT "articles".*
FROM "articles"
LEFT OUTER JOIN "mentions" ON "mentions"."article_id" = "articles"."id"
LEFT OUTER JOIN "users" ON "users"."id" = "mentions"."user_id"
WHERE "users"."name" = 'pippo'
```

and `:subquery` drops the `DISTINCT` by wrapping the join in an `IN (SELECT
...)`:

```sql
SELECT "articles".*
FROM "articles"
WHERE "articles"."id" IN (
  SELECT "articles"."id" FROM "articles"
  LEFT OUTER JOIN "mentions" ON ...
  LEFT OUTER JOIN "users" ON ...
  WHERE "users"."name" = 'pippo'
)
```

## What the port does today

All SQL adapters render `can` conditions joined with `OR`, `cannot`
conditions with `AND NOT`, values as bind parameters. For
[`Nested` conditions](./hash_of_conditions.md#traversing-associations)
(association traversal - the gem's JOIN case) the adapters differ:

| adapter | nested-condition behavior |
|---|---|
| `cancancan-diesel` (typed) | `WrongAssociation` error - join metadata cannot be inferred from the condition alone |
| `cancancan-diesel` (fragment) | renders the raw fragment against the relation name (you control the JOIN in the query) |
| `cancancan-sqlx` | `WrongAssociation` error (same rationale as typed Diesel) |
| `cancancan-seaorm` | `WrongAssociation` error (same rationale) |
| `cancancan-mongo` | flattens into dot-notation (`author.name`) - natural for embedded documents |

So today, association-traversing rules on SQL backends are expressed with
[`RawSql` conditions](./hash_of_conditions.md#raw-sql-fragments) over a query
that already declares its joins. A pluggable `accessible_by_strategy`
(`:left_join` / `:subquery` equivalents) is tracked in
[PARITY.md](./PARITY.md) and lands with a future adapter release.
