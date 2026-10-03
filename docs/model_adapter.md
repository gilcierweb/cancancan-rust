# Model adapters

`cancancan-rust` ships with maintained adapters, each translating
[`Condition`] rules into the native query language of one backend:

| crate              | backend                          | state        |
|--------------------|----------------------------------|--------------|
| `cancancan-diesel` | Diesel 2.x (typed + raw fragment)| stable       |
| `cancancan-seaorm` | SeaORM / sea-query               | builder-level|
| `cancancan-sqlx`   | SQLx query builder               | stable       |
| `cancancan-mongo`  | MongoDB (`bson` documents)       | stable       |

See [query adapters](./query-adapters.md) for usage. This page is about
writing **your own** adapter — the port of the gem's `AbstractAdapter`.

## What an adapter does

An adapter answers one question: *given the rules relevant to an
`(action, subject)` pair, produce a query that returns exactly the authorized
rows.* The pipeline is always:

1. `ability.rules_for_query(action, subject_type)` — the surviving rules
   (cannot-rules, matchers and raw-SQL handling already applied; see
   [rules](./rules.md)). Rules arrive in precedence order.
2. Optionally `cancancan_core::compress(rules)` when
   `rules_compressor_enabled()` is on — see
   [rules compression](./rules_compression.md).
3. Translate each rule's `Condition` tree into the backend's filter DSL.
4. Combine: `can` rules OR together, `cannot` rules subtract (AND NOT),
   applied in order.

And the invariant from the gem, unchanged: **if no rules match the
`(action, subject)` pair, the adapter must return a query that yields zero
rows** — never an unfiltered `SELECT *`.

## A minimal custom adapter

Any Rust type can expose an `accessible_by(&Ability, &str)` entry point. The
translation work is a recursive walk over `Condition`:

```rust
use cancancan_core::{Ability, Condition, DbValue, compress, rules_compressor_enabled};

pub fn filter_for(ability: &Ability, action: &str, subject: &str) -> MyFilter {
    let rules = ability.rules_for_query(action, subject)
        .expect("cannot/matcher rules require care"); // see rules_for_query docs

    let rules = if rules_compressor_enabled() { compress(rules) } else { rules };

    let mut filter = MyFilter::none();
    for rule in rules {
        let f = translate(rule.condition());
        if rule.is_can() { filter = filter.or(f); } else { filter = filter.and_not(f); }
    }
    filter
}

fn translate(c: &Condition) -> MyFilter {
    match c {
        Condition::All => MyFilter::all(),
        Condition::Eq { field, value } => MyFilter::eq(field, value),
        Condition::Ne { field, value } => MyFilter::ne(field, value),
        Condition::In { field, values } => MyFilter::is_in(field, values),
        Condition::Range { field, min, max } => MyFilter::range(field, min, max),
        Condition::IsNull { field, is_null } => MyFilter::is_null(field, *is_null),
        Condition::And(cs) => cs.iter().map(translate).fold(MyFilter::all(), MyFilter::and),
        Condition::Or(cs) => cs.iter().map(translate).fold(MyFilter::none(), MyFilter::or),
        Condition::Not(inner) => translate(inner).negate(),
        Condition::Nested { relation, condition } => MyFilter::join(relation, translate(condition)),
        Condition::RawSql(sql) => MyFilter::raw(sql),
    }
}
```

## Testing an adapter

Follow the gem's TDD recipe (its example is the Mongoid adapter):

1. **Empty-rule behavior** — no rules ⇒ zero rows.
2. **Round-trip** — insert rows, run the generated query, assert the result
   equals the in-memory `can_check` verdict for every row.
3. **Cannot rules** — a broad `can` followed by `cannot` must exclude the
   denied subset.

```rust
#[test]
fn returns_only_accessible_rows() {
    let mut ability = Ability::new();
    ability.can_where(Some("read"), Some("Project"),
        Condition::Eq { field: "title".into(), value: DbValue::Str("Sir".into()) })
        .unwrap();

    insert_project("Sir");
    insert_project("Lord");

    let titles = run(filter_for(&ability, "read", "Project"));
    assert_eq!(titles, vec!["Sir"]);
}
```

The shipped adapters (`cancancan-diesel`'s typed and fragment paths, the
`cancancan-mongo` BSON translator) are the reference implementations — read
them before starting a new backend.
