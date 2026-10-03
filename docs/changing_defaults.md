# Changing defaults

If you need to use a different naming strategy for your `Ability` (the gem
calls this an `Ability`/`CanCan::Ability` hook), you have full control in
Rust: the `Ability` is constructed in your code with rules declared inline
or through helper functions).

There is no `alias_action`-level default modification; write your alias
definitions at ability build time, or wrap them.

## Custom default message

The only forwarded pattern at a system level is:

```rust
let mut ability = Ability::new();
ability.set_message_resolver(Arc::new(|action, subject| {
    // Translate by resolver → I18n ("You are not authorized to...")
}));
```

This mirrors the Rails convention of putting the denial message logic in
`Ability#initialize` or the controller.
