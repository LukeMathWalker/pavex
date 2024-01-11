use itertools::Itertools;
use pavex_bp_schema::MethodGuard;

/// A `RouterKey` uniquely identifies a subset of incoming requests for routing purposes.
/// Each request handler is associated with a `RouterKey`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouterKey {
    pub path: String,
    pub method_guard: MethodGuard,
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
