use std::collections::BTreeMap;
use std::fmt::Formatter;
use std::fmt::Write;

use ahash::HashMap;
use bimap::BiHashMap;
use guppy::PackageId;

use crate::language::{NamedTypeGeneric, ResolvedPath, ResolvedType};

#[derive(Clone, Hash, Eq, PartialEq)]
/// A Rust type that can be invoked - e.g. a function, a method, a struct literal constructor.
pub(crate) struct Callable {
    /// `true` if the callable declaration uses the `async` keyword.
    ///
    /// # Implementation Gaps
    ///
    /// It is **NOT** set to `true` if the function does not use the `async` keyword but returns
    /// a type that implements the `Future` trait.
    pub is_async: bool,
    /// `None` if the callable returns the unit type (`()`).
    pub output: Option<ResolvedType>,
    /// The fully-qualified path pointing at this callable.
    pub path: ResolvedPath,
    /// The types of the callable input parameter types.
    /// The list is ordered, matching the order in the callable declaration - this is relevant
    /// to ensure correct invocations.
    pub inputs: Vec<ResolvedType>,
    /// Rust supports different types of callables which rely on different invocation syntax.
    /// See [`InvocationStyle`] for more details.
    pub invocation_style: InvocationStyle,
}

impl Callable {
    /// Replace all unassigned generic type parameters in this callable with the
    /// concrete types specified in `bindings`.
    ///
    /// The newly "bound" callable will be returned.
    pub fn bind_generic_type_parameters(
        &self,
        bindings: &HashMap<NamedTypeGeneric, ResolvedType>,
    ) -> Callable {
        // TODO: we should bind the generics on the path of the callable itself.
        let inputs = self
            .inputs
            .iter()
            .map(|t| t.bind_generic_type_parameters(bindings))
            .collect();
        let output = self
            .output
            .as_ref()
            .map(|t| t.bind_generic_type_parameters(bindings));
        Self {
            output,
            inputs,
            ..self.clone()
        }
    }
}

/// Rust supports different types of callables which rely on different invocation syntax.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) enum InvocationStyle {
    /// `<callable_path>(<comma-separated list of input parameters)`.
    /// Used by functions and methods. The latter is only valid if the callable path
    /// includes the name of the item that the method is attached to (e.g. `MyStruct::init()` is
    /// valid, while `init()` will not point at the method even if `MyStruct` is in scope).
    FunctionCall,
    /// `<struct_name> { <field_name>: <field_value>, ...}`
    /// An available option to build structs **if all their fields are public**.
    StructLiteral {
        /// A map associating each field name to its type.
        field_names: BTreeMap<String, ResolvedType>,
    },
}

impl Callable {
    pub fn render_signature(&self, package_ids2names: &BiHashMap<PackageId, String>) -> String {
        let mut buffer = String::new();
        write!(&mut buffer, "{}", self.path).unwrap();
        write!(&mut buffer, "(").unwrap();
        let mut inputs = self.inputs.iter().peekable();
        while let Some(input) = inputs.next() {
            write!(&mut buffer, "{}", input.render_type(package_ids2names)).unwrap();
            if inputs.peek().is_some() {
                write!(&mut buffer, ", ").unwrap();
            }
        }
        write!(&mut buffer, ")",).unwrap();
        if let Some(output) = &self.output {
            write!(&mut buffer, " -> {}", output.render_type(package_ids2names)).unwrap();
        }
        buffer
    }
}

impl std::fmt::Debug for Callable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)?;
        write!(f, "(")?;
        let mut inputs = self.inputs.iter().peekable();
        while let Some(input) = inputs.next() {
            write!(f, "{input:?}")?;
            if inputs.peek().is_some() {
                write!(f, ", ")?;
            }
        }
        if let Some(output) = &self.output {
            write!(f, ") -> {output:?}")?;
        }
        Ok(())
    }
}
