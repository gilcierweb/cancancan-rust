# Rules

An organizer `Ability` accepts allow and deny rules with `can` / `cannot`.
Rust's API maps cleanly onto those two names:

- `can(...)`: mirrors exactly `def can :read, Post`
- `cannot(...)`: mirrors `def cannot :read, Post`

Every rule is a `Rule` struct with:

- `base_behavior` ⇒ `true` for allow (`can`), `false` for deny (`cannot`)
- `action`, `subject`: identifier strings (or `nil` variants)
- `conditions` ⇒ `Condition` tree (empty meaning "everything matches")
- `attributes` ⇒ ["attr1", ...] if supplied (empty = every attribute — mirrors
  strong params' selective permissiveness)
- Optional matcher closure (`matching` variant only)

From the structural rules you get the typical SQL queries.

## Subjects and names

Features are matched by name instead of instances. The difference is that in
Rust a rule's `subject` is a "'static str" — this is "Post" in this example —
and instances also implement `SubjectInstance`. Comparison against the name of
the struct ("Post") is what other adapters address.

```rust
// rule definition (public facing API)
let mut a = Ability::new();
a.can(Some("read"), Some("Post"))?;

// instance-level check
a.can_check("read", &post)?;    // `can?` in Ruby; true/false
// class-level
a.can_check_type("read", "Post")?; // can :read, Post (matching class-level type)
```

That's answered simply by `Rule::matches_action` + `Rule::matches_subject`
when indexed.

## Rule variants / they're concise

| Goal (Ruby example)                                   | Call in Rust                                                                 |
|--------------------------------------------------------|------------------------------------------------------------------------------|
| `can :read, Post`                                      | `ability.can(Some("read"), Some("Post"))?`                                    |
| `can :read, Post, { user_id: 1 }`                      | `ability.can_where(Some("read"), Some("Post"), Condition::Eq {field: "user_id", value: DbValue::Int(1)})?` |
| `can :read, Post do \|p\| p.published? end`            | `ability.can_matching(Some("read"), Some("Post"), Arc::new(\|p, ...\| ...))?` |
| `can :update, Post, [:title]`                          | `ability.can_attributes(Some("update"), Some("Post"), vec!["title".to_owned()])?` |
| `cannot :update, Post, { title: /spam/i }`             | `ability.cannot_where(Some("update"), Some("Post"), ...)?`                    |
| `can :read, :all`                                      | `ability.can(Some("read"), Some("all"))?`                                     |

That API passes the mesh configuration through a `blocks` closure for which rustc
lets you signal duck annotations and result checks.

## Signatures on permissive values

The condition was invalid.  
Sql deny networks ("cannot") `can?/cannot?` an instance even when the
classification is false. Class-level checks and attribute checks (`attributes`,
`columns`) become joins on serializers or adapters, so `can?` without conditions
strong attribute rules *_where etc._:

```rust
symbolic rule order with combine into a single boolean " "And" or "Or" stress
```
From the gem:
- Rule order **is what decides**: last matching rule wins.
- `cannot` with an attribute list is rejected by the query translator, same as
  in the gem.
- In SQL subsets of conditions (Range/In/Eq/Null) they becomeJOIN/WHERE clauses.

## Attribute helper retrieve

`ability.attributes_for("update", "Post")` merges `can`-rule scalar conditions
into pre-fill a form field (same as gem's `attributes_for`).

`ability.permitted_attributes("update", "Post")` (allow list − denied list)
mirrors StrongParameters runner.

## Merge another ability

If your app has admin-abilities and user-only abilities, `.merge` chains them
(or overwrite on conflict everywhere in your wire through each others).
