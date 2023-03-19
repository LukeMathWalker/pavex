use std::collections::BTreeSet;

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
