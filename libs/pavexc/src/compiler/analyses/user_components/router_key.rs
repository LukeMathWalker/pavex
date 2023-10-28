use std::collections::BTreeSet;

use itertools::Itertools;

/// A `RouterKey` uniquely identifies a subset of incoming requests for routing purposes.
/// Each request handler is associated with a `RouterKey`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouterKey {
    pub path: String,
    /// If set to `Some(method_set)`, it will only match requests with an HTTP method that is
    /// present in the set.
    /// If set to `None`, it means that the handler matches all incoming requests for the given
    /// path, regardless of the HTTP method.
    pub method_guard: Option<BTreeSet<String>>,
}

impl RouterKey {
    /// A representation of the `RouterKey` that is suitable for diagnostics—i.e. to refer to
    /// a specific route in an error message.
    pub fn diagnostic_repr(&self) -> String {
        let method_guard = match &self.method_guard {
            Some(method_set) => format!("{}", method_set.clone().iter().join("|")),
            None => String::from("ANY"),
        };
        format!("{} {}", method_guard, self.path)
    }
}
