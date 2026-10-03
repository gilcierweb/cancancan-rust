# Inherited Resources / nested resources

> **Status: not ported — Rails-specific by design.**

The gem's `inherited_resources.md` and `nested_resources.md` describe the
integration with the InheritedResources Rails controller pattern and with
shallow/nested Rails routes (`load_and_authorize_resource :project` +
`:task, through: :project`). Both exist to bridge CanCanCan with
Rails-specific controller stacks.

`cancancan-rust` targets Rust web frameworks (axum, actix-web) where route
nesting and resource loading are explicit handler/extractor code — there is
no equivalent controller superclass to integrate with.

## The port's answer to nested authorization

In Rails you'd write:

```ruby
load_and_authorize_resource :project
load_and_authorize_resource :task, through: :project
```

The port makes the same two steps explicit, using a `Nested` condition so the
check traverses the association (see
[hash of conditions](./hash_of_conditions.md#traversing-associations)):

```rust
// rules: task management flows through the project's owner
ability.can_where(
    Some("manage"),
    Some("Task"),
    Condition::Nested {
        relation: "project".into(),
        condition: Box::new(Condition::Eq {
            field: "user_id".into(),
            value: DbValue::Int(user.id),
        }),
    },
)?;
```

```rust
// axum handler for POST /projects/{project_id}/tasks
async fn create_task(
    ability: CurrentAbility,
    Path(project_id): Path<i64>,
    Json(input): Json<TaskInput>,
) -> Result<Json<Task>, AppError> {
    let project = load_project(project_id)?;          // 404 if absent
    ability.authorize("read", &project)?;             // parent gate

    let task = input.into_task(project.id);
    ability.authorize("create", &task)?;              // nested rule via `project`
    Ok(Json(task.insert(&mut conn)?))
}
```

The pieces map directly:

| gem | port |
|---|---|
| `load_and_authorize_resource :project` | `load_project` + `authorize("read", &project)` |
| `through: :project` | `Condition::Nested { relation: "project", ... }` on the `Task` rules |
| shallow routes | your router's normal path nesting |

See [web integration](./web-integration.md) for the
`authorize` / `load_and_authorize` extractors available in axum and actix.
