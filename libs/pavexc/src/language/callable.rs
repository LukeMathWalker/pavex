use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Formatter;
use std::fmt::Write;

use ahash::HashMap;
use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexSet;

use crate::language::{ResolvedPath, ResolvedType};
use crate::rustdoc::GlobalItemId;

#[derive(Clone, Hash, Eq, PartialEq)]
/// A Rust type that can be invoked—e.g. a function, a method, a struct literal constructor.
pub struct Callable {
    /// `true` if the callable declaration uses the `async` keyword.
    ///
    /// # Implementation Gaps
    ///
    /// It is **NOT** set to `true` if the function doesn't use the `async` keyword but returns
    /// a type that implements the `Future` trait.
    pub is_async: bool,
    /// `true` if the first input parameter to the callable is `&self` or `&mut self`.  
    /// This is relevant to determine borrow relationships between the callable inputs and outputs
    /// in case some lifetime parameters were elided.
    ///
    /// See https://doc.rust-lang.org/nomicon/lifetime-elision.html for more details.
    pub takes_self_as_ref: bool,
    /// `None` if the callable returns the unit type (`()`).
    /// Otherwise, the type of the callable return value.
    pub output: Option<ResolvedType>,
    /// The fully-qualified path pointing at this callable.
    ///
    /// E.g. `std::vec::Vec::new` for `Vec::new()`.
    pub path: ResolvedPath,
    /// The types of the callable input parameter types.
    /// The list is ordered, matching the order in the callable declaration—this is relevant
    /// to ensure correct invocations.
    pub inputs: Vec<ResolvedType>,
    /// Rust supports different types of callables which rely on different invocation syntax.
    /// See [`InvocationStyle`] for more details.
    pub invocation_style: InvocationStyle,
    /// The ids required to locate this callable in the JSON docs for the package where it is
    /// defined.
    ///
    /// It is optional to allow for flexible usage patterns—e.g. to leverage [`Callable`]
    /// to work with callables that we want to code-generate into a new crate.  
    pub source_coordinates: Option<GlobalItemId>,
}

impl Callable {
    /// Replace all unassigned generic type parameters in this callable with the
    /// concrete types specified in `bindings`.
    ///
    /// The newly "bound" callable will be returned.
    pub fn bind_generic_type_parameters(
        &self,
        bindings: &HashMap<String, ResolvedType>,
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

    /// Returns the set of all unassigned generic type parameters in this callable.
    ///
    /// E.g. `[T]` for `fn f<T>() -> Json<T, u8>` or `[T, V]` for `fn g<T, V>() -> Json<T, V>`.
    #[allow(unused)]
    pub(crate) fn unassigned_generic_type_parameters(&self) -> IndexSet<String> {
        let mut result = IndexSet::new();
        for input in &self.inputs {
            result.extend(input.unassigned_generic_type_parameters());
        }
        if let Some(output) = &self.output {
            result.extend(output.unassigned_generic_type_parameters());
        }
        result
    }

    /// Returns `true` if this callable is fallible—i.e. if it returns a `Result` type.
    pub fn is_fallible(&self) -> bool {
        if let Some(output) = &self.output {
            output.is_result()
        } else {
            false
        }
    }

    /// Returns a new [`Callable`] where all lifetime parameters in the
    /// output type (if present) are explicitly named.  
    ///
    /// This is relevant to ensure correct borrow relationships between the callable
    /// inputs and outputs in case some lifetime parameters were elided.
    pub(crate) fn unelide_output_lifetimes(&self) -> Self {
        if self.invocation_style != InvocationStyle::FunctionCall {
            // Struct literals don't have lifetime elision.
            return self.clone();
        }

        let Some(output) = &self.output else {
            return self.clone();
        };
        if !output.has_implicit_lifetime_parameters() {
            return self.clone();
        }

        let mut elided_output_lifetime: String = "elided".to_string();
        let mut inputs = self.inputs.clone();
        for input in inputs.iter_mut() {
            if input.has_implicit_lifetime_parameters() {
                elided_output_lifetime = {
                    let mut named_lifetime_parameters = BTreeSet::new();
                    for input in &self.inputs {
                        named_lifetime_parameters.extend(input.named_lifetime_parameters());
                    }
                    named_lifetime_parameters.extend(output.named_lifetime_parameters());

                    if named_lifetime_parameters.contains("elided") {
                        // Unlucky! Let's craft a lifetime name that for sure
                        // doesn't belong to the set, leveraging that the set
                        // is ordered in ascending order.
                        let last = named_lifetime_parameters.last().unwrap();
                        format!("{last}_")
                    } else {
                        "elided".to_string()
                    }
                };

                input.set_implicit_lifetimes(elided_output_lifetime.clone());
                break;
            }

            let named_params = input.named_lifetime_parameters();
            if !named_params.is_empty() {
                elided_output_lifetime = named_params.first().unwrap().to_string();
                break;
            }
        }

        let mut output = output.clone();
        output.set_implicit_lifetimes(elided_output_lifetime);

        Self {
            is_async: self.is_async,
            takes_self_as_ref: self.takes_self_as_ref,
            output: Some(output),
            path: self.path.clone(),
            inputs,
            invocation_style: self.invocation_style.clone(),
            source_coordinates: self.source_coordinates.clone(),
        }
    }

    /// Returns the indices of all input parameters that the output type
    /// borrows from.
    ///
    /// E.g. `fn f(x: &T) -> &T` returns `[0]`.
    pub(crate) fn inputs_that_output_borrows_from(&self) -> Vec<usize> {
        let c = self.unelide_output_lifetimes();
        let Some(output) = &c.output else {
            return vec![];
        };

        let output_lifetime_parameters = output.named_lifetime_parameters();

        let mut borrowed_indexes = vec![];
        for (i, input) in c.inputs.iter().enumerate() {
            if input
                .named_lifetime_parameters()
                .intersection(&output_lifetime_parameters)
                .next()
                .is_some()
            {
                borrowed_indexes.push(i);
            }
        }
        borrowed_indexes
    }
}

/// Rust supports different types of callables which rely on different invocation syntax.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum InvocationStyle {
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
        /// Rust does not have default values for struct fields.
        /// This is hack to allow us to inject the `next` field in the state we generate for
        /// `Next` where the `next` field is not part of the struct definition and it must
        /// be set to a pre-determined function pointer in order to work around the lack of
        /// TAIT on stable.
        /// TODO: remove when TAIT stabilizes.
        extra_field2default_value: BTreeMap<String, String>,
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
