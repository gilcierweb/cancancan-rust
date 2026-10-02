use std::collections::{HashMap, HashSet};

/// Registry of action aliases (e.g. `read` expands to `index` and `show`).
///
/// Mirrors `CanCan::Ability::Actions` from the Ruby gem. The default aliases
/// match the gem: `read` to `index`/`show`, `create` to `new`, `update` to `edit`.
#[derive(Debug, Clone)]
pub struct Actions {
    aliases: HashMap<String, Vec<String>>,
}

impl Actions {
    /// Creates a registry preloaded with the gem default aliases.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            aliases: HashMap::new(),
        };
        registry.alias_action(["index", "show"], "read");
        registry.alias_action(["new"], "create");
        registry.alias_action(["edit"], "update");
        registry
    }

    /// Maps one or more actions to a single alias target.
    pub fn alias_action<I, S>(&mut self, actions: I, target: S)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let entry = self.aliases.entry(target.into()).or_default();
        for action in actions {
            let action = action.into();
            if !entry.contains(&action) {
                entry.push(action);
            }
        }
    }

    /// Expands an action into itself plus every transitively aliased action.
    #[must_use]
    pub fn expand(&self, action: &str) -> Vec<String> {
        let mut expanded = vec![action.to_owned()];
        let mut visited = HashSet::from([action.to_owned()]);
        let mut queue = vec![action.to_owned()];
        while let Some(current) = queue.pop() {
            if let Some(mapped) = self.aliases.get(&current) {
                for next in mapped {
                    if visited.insert(next.clone()) {
                        expanded.push(next.clone());
                        queue.push(next.clone());
                    }
                }
            }
        }
        expanded
    }

    /// Returns every alias name that (transitively) expands to `action`.
    #[must_use]
    pub fn aliases_for(&self, action: &str) -> Vec<String> {
        self.aliases
            .keys()
            .filter(|target| {
                target.as_str() != action && self.expand(target).contains(&action.to_owned())
            })
            .cloned()
            .collect()
    }

    /// Removes every registered alias, including the defaults.
    pub fn clear(&mut self) {
        self.aliases.clear();
    }
}

impl Default for Actions {
    fn default() -> Self {
        Self::new()
    }
}
