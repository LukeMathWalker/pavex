use std::collections::BTreeSet;

use itertools::Itertools;

/// A `RouterKey` uniquely identifies a subset of incoming requests for routing purposes.
/// Each request handler is associated with a `RouterKey`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouterKey {
    pub path: String,
    pub method_guard: MethodGuard,
}

/// If set to `Some(method_set)`, it will only match requests with an HTTP method that is
/// present in the set.
/// If set to `Any`, it means that the handler matches all incoming requests for the given
/// path, regardless of the HTTP method.
/// Custom methods are only allowed if `with_extensions` is set to `true`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MethodGuard {
    Any,
    Some(BTreeSet<String>),
}

impl RouterKey {
    /// A representation of the `RouterKey` that is suitable for diagnostics—i.e. to refer to
    /// a specific route in an error message.
    pub fn diagnostic_repr(&self) -> String {
        let method_guard = match &self.method_guard {
            MethodGuard::Any => String::from("*"),
            MethodGuard::Some(method_set) => method_set.clone().iter().join("|").to_string(),
        };
        format!("{} {}", method_guard, self.path)
    }
}
