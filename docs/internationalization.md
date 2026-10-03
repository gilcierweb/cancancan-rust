# Internationalization

The gem resolves denial messages through I18n YAML files. The port keeps the
same lookup semantics but leaves the i18n engine to you: translations plug in
through a **message resolver** closure.

```rust
use cancancan_core::Ability;

let mut ability = Ability::new();
ability.set_message_resolver(|action, subject| {
    // delegate to your i18n stack (fluent, rust-i18n, simple maps, ...)
    let key = format!("unauthorized.{action}.{subject}");
    my_i18n::lookup(&key)
});
```

The resolved message surfaces in
[`CanCanError::AccessDenied`](./handling_access_denied.md) and through
`ability.unauthorized_message(action, subject)`.

## Lookup order

Mirroring the gem's YAML nesting - from most specific to most general:

```yaml
# gem equivalent, for reference
en:
  unauthorized:
    manage:
      all: "You have no access to this resource"
    create:
      article: "Only an admin can create an article"
    vote:
      article: "Only users with one or more articles can vote"
```

A resolver that reproduces this fall-back chain:

```rust
ability.set_message_resolver(|action, subject| {
    my_i18n::lookup(&format!("unauthorized.{action}.{subject}"))
        .or_else(|| my_i18n::lookup(&format!("unauthorized.{action}.all")))
        .or_else(|| my_i18n::lookup(&format!("unauthorized.manage.{subject}")))
        .or_else(|| my_i18n::lookup("unauthorized.manage.all"))
});
```

Custom actions work the same way - `("vote", "Article")` is just another
key. When no key resolves, the default message is used:
`"You are not authorized to access this page."` (the gem's default).

## Variables in messages

The gem interpolates `%{action}` / `%{subject}`. If your i18n library does
interpolation, pass the variables; otherwise a plain `replace` is enough:

```rust
ability.set_message_resolver(|action, subject| {
    my_i18n::lookup("unauthorized.manage.all").map(|t| {
        t.replace("%{action}", action).replace("%{subject}", subject)
    })
});
```

The resolver runs per ability, so different user locales are simply different
resolvers - build the ability with the request's locale in scope:

```rust
let ability = ability_for(&user).with_locale(&request_locale);
```

(where `with_locale` is your own wrapper that installs the resolver above
with the right catalog).
