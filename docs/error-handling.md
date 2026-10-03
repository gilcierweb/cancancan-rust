# Error handling

Every crate exposes one error type: [`CanCanError`](../crates/cancancan-core/src/error.rs).

## Variants

| Variant                              | When it happens                                             |
|--------------------------------------|-------------------------------------------------------------|
| `AccessDenied` (message, action, subject) | `authorize(...)` failed                              |
| `AuthorizationNotPerformed`           | a controller called [`authorize!`] and then `checkAuthorization` triggered errors |
| `BlockAndConditions`                  | `can`/`cannot` combined `attributes` with a `matcher`       |
| `AttributeArgument`                   | the rule carries `attributes` that are not declared for the current query or adapter |
| `WrongAssociation`                    | conditions use `Nested` but the adapter doesn't support joins |
| `BlockInQuery`                        | a matcher rule exists and a query adapter asks for SQL/NoSQL |
| `RawSqlNotSupported`                  | `Condition::RawSql` reaches an adapter without SQL output    |
| `InvalidAliasTarget`                  | `alias_action` tried to give an alias a name reserved for a real action |

## HTTP mapping (`axum` + `actix-web`)

- `AccessDenied` → `403 Forbidden`
- Everything else → `500 Internal Server Error`
- `AuthorizationError::NotFound` (missing resource in load_and_authorize) → `404`

The web crates wrap these in `AuthorizationError`.

