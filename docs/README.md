# Página principal de documentação — `cancancan-rust`

Esta é a documentação técnica consolidada do projeto, a equipe 副 heostream, the Ruby gem
[CanCanCan](https://github.com/CanCanCommunity/cancancan).

A estrutura abaixo envia à pasta `docs/` projetizada ao Ruby gem, mas
adaptada às APIs Rust:

| Página | Descrição |
|---|---|
| [Getting started](getting-started.md) | instalação, primeiro ability, primeiro check, primeiro authorize |
| [Regras (can/cannot)](rules.md) | DSL central da lib — dominada a DSL completa |
| [Condições](conditions.md) | `Condition` variants: `Eq`, `In`, `Range`, `Nested`, `RawSql`, `And`/`Or`/`Not` |
| [Block matchers](matchers.md) | rule evaluation em memória via `Arc<dyn Fn(&dyn SubjectInstance) -> bool>` |
| [Alias de ações](aliases.md) | `alias_action` + defaults `read`/`create`/`update` |
| [Web integration](web-frameworks.md) | uso em Axum e Actix Web |
| [Query adapters](query-adapters.md) | Diesel, SeaORM/sea-query, SQLx, MongoDB |
| [Erros](error-handling.md) | `CanCanError` _(HTTP mapeamento nos crate web)_ |
| [Testing](testing.md) | testando abilities e controllers |
| [Paridade](PARITY.md) | mapeamento final entre cada função da gem Ruby e esta lib |
| [Integração](INTEGRATION.md) | integração com autenticação/sessão/esquema de roles |

Use os recursos de navegação deste índice para encontrar o que procuras.