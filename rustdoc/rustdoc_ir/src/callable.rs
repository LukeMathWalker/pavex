use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Formatter;
use std::fmt::Write;

use ahash::HashMap;
use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexSet;

use crate::{
    EnumVariantConstructorPath, FreeFunctionPath, InherentMethodPath, Lifetime, StructLiteralPath,
    TraitMethodPath, Type,
};
use rustdoc_ext::GlobalItemId;

/// A valid Rust identifier.
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct RustIdentifier(String);

impl RustIdentifier {
    /// Create a new [`RustIdentifier`].
    ///
    /// # Panics
    ///
    /// Panics if `name` is not a valid Rust identifier (`[a-zA-Z_][a-zA-Z0-9_]*`).
    pub fn new(name: String) -> Self {
        assert!(
            Self::is_valid_identifier(&name),
            "Invalid identifier: `{name}`"
        );
        Self(name)
    }

    /// Returns `true` if `name` matches `[a-zA-Z_][a-zA-Z0-9_]*`.
    fn is_valid_identifier(name: &str) -> bool {
        let mut chars = name.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
            _ => return false,
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RustIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A named input parameter of a [`Callable`].
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct CallableInput {
    pub name: RustIdentifier,
    pub type_: Type,
}

// ── Shared pieces ──────────────────────────────────────────

/// Fields specific to callables that use function-call syntax.
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct FnHeader {
    pub output: Option<Type>,
    pub inputs: Vec<CallableInput>,
    pub is_async: bool,
    pub abi: rustdoc_types::Abi,
    pub is_unsafe: bool,
    pub is_c_variadic: bool,
    pub symbol_name: Option<String>,
}

// ── Per-variant structs ────────────────────────────────────

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct FreeFunction {
    pub path: FreeFunctionPath,
    pub header: FnHeader,
    pub source_coordinates: Option<GlobalItemId>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct InherentMethod {
    pub path: InherentMethodPath,
    pub header: FnHeader,
    pub source_coordinates: Option<GlobalItemId>,
    pub takes_self_as_ref: bool,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct TraitMethod {
    pub path: TraitMethodPath,
    pub header: FnHeader,
    pub source_coordinates: Option<GlobalItemId>,
    pub takes_self_as_ref: bool,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct StructLiteralInit {
    pub path: StructLiteralPath,
    pub self_: Option<Type>,
    pub fields: Vec<CallableInput>,
    pub source_coordinates: Option<GlobalItemId>,
    /// Extra fields injected during codegen (e.g. `next` for middleware state).
    pub extra_field2default_value: BTreeMap<String, String>,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct EnumVariantInit {
    pub path: EnumVariantConstructorPath,
    pub self_: Option<Type>,
    pub fields: Vec<CallableInput>,
    pub source_coordinates: Option<GlobalItemId>,
}

// ── The enum ───────────────────────────────────────────────

/// A Rust type that can be invoked—e.g. a function, a method, a struct literal constructor.
#[derive(Clone, Hash, Eq, PartialEq)]
pub enum Callable {
    FreeFunction(FreeFunction),
    InherentMethod(InherentMethod),
    TraitMethod(TraitMethod),
    StructLiteralInit(StructLiteralInit),
    EnumVariantInit(EnumVariantInit),
}

impl Callable {
    // ── Accessors ──────────────────────────────────────────

    pub fn output(&self) -> Option<&Type> {
        match self {
            Callable::FreeFunction(f) => f.header.output.as_ref(),
            Callable::InherentMethod(m) => m.header.output.as_ref(),
            Callable::TraitMethod(m) => m.header.output.as_ref(),
            Callable::StructLiteralInit(s) => s.self_.as_ref(),
            Callable::EnumVariantInit(e) => e.self_.as_ref(),
        }
    }

    pub fn inputs(&self) -> &[CallableInput] {
        match self {
            Callable::FreeFunction(f) => &f.header.inputs,
            Callable::InherentMethod(m) => &m.header.inputs,
            Callable::TraitMethod(m) => &m.header.inputs,
            Callable::StructLiteralInit(s) => &s.fields,
            Callable::EnumVariantInit(e) => &e.fields,
        }
    }

    pub fn source_coordinates(&self) -> Option<&GlobalItemId> {
        match self {
            Callable::FreeFunction(f) => f.source_coordinates.as_ref(),
            Callable::InherentMethod(m) => m.source_coordinates.as_ref(),
            Callable::TraitMethod(m) => m.source_coordinates.as_ref(),
            Callable::StructLiteralInit(s) => s.source_coordinates.as_ref(),
            Callable::EnumVariantInit(e) => e.source_coordinates.as_ref(),
        }
    }

    /// Returns an iterator over the types of the input parameters.
    pub fn input_types(&self) -> impl Iterator<Item = &Type> + '_ {
        self.inputs().iter().map(|i| &i.type_)
    }

    /// Returns `true` if this callable is fallible—i.e. if it returns a `Result` type.
    pub fn is_fallible(&self) -> bool {
        if let Some(output) = self.output() {
            output.is_result()
        } else {
            false
        }
    }

    /// Returns `true` if the callable declaration uses the `async` keyword.
    pub fn is_async(&self) -> bool {
        match self {
            Callable::FreeFunction(f) => f.header.is_async,
            Callable::InherentMethod(m) => m.header.is_async,
            Callable::TraitMethod(m) => m.header.is_async,
            Callable::StructLiteralInit(_) | Callable::EnumVariantInit(_) => false,
        }
    }

    // ── Path accessors ─────────────────────────────────────

    pub fn package_id(&self) -> &PackageId {
        match self {
            Callable::FreeFunction(f) => &f.path.package_id,
            Callable::InherentMethod(m) => &m.path.package_id,
            Callable::TraitMethod(m) => &m.path.package_id,
            Callable::StructLiteralInit(s) => &s.path.package_id,
            Callable::EnumVariantInit(e) => &e.path.package_id,
        }
    }

    pub fn render_as_expression_path(
        &self,
        id2name: &BiHashMap<PackageId, String>,
        buffer: &mut String,
    ) {
        match self {
            Callable::FreeFunction(f) => f.path.render_path(id2name, buffer),
            Callable::InherentMethod(m) => m.path.render_path(id2name, buffer),
            Callable::TraitMethod(m) => m.path.render_path(id2name, buffer),
            Callable::StructLiteralInit(s) => s.path.render_path(id2name, buffer),
            Callable::EnumVariantInit(e) => e.path.render_path(id2name, buffer),
        }
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        match self {
            Callable::FreeFunction(f) => f.path.render_for_error(buffer),
            Callable::InherentMethod(m) => m.path.render_for_error(buffer),
            Callable::TraitMethod(m) => m.path.render_for_error(buffer),
            Callable::StructLiteralInit(s) => s.path.render_for_error(buffer),
            Callable::EnumVariantInit(e) => e.path.render_for_error(buffer),
        }
    }

    // ── Existing methods ───────────────────────────────────

    /// Returns the set of all unassigned generic type parameters in this callable.
    #[allow(unused)]
    pub fn unassigned_generic_type_parameters(&self) -> IndexSet<String> {
        let mut result = IndexSet::new();
        for input in self.input_types() {
            result.extend(input.unassigned_generic_type_parameters());
        }
        if let Some(output) = self.output() {
            result.extend(output.unassigned_generic_type_parameters());
        }
        result
    }

    /// Replace all unassigned generic type parameters in this callable with the
    /// concrete types specified in `bindings`.
    pub fn bind_generic_type_parameters(&self, bindings: &HashMap<String, Type>) -> Callable {
        let inputs: Vec<CallableInput> = self
            .inputs()
            .iter()
            .map(|i| CallableInput {
                name: i.name.clone(),
                type_: i.type_.bind_generic_type_parameters(bindings),
            })
            .collect();
        let output = self
            .output()
            .map(|t| t.bind_generic_type_parameters(bindings));
        let mut result = self.clone();
        match &mut result {
            Callable::FreeFunction(f) => {
                f.header.inputs = inputs;
                f.header.output = output;
            }
            Callable::InherentMethod(m) => {
                m.header.inputs = inputs;
                m.header.output = output;
            }
            Callable::TraitMethod(m) => {
                m.header.inputs = inputs;
                m.header.output = output;
            }
            Callable::StructLiteralInit(s) => {
                s.fields = inputs;
                s.self_ = output;
            }
            Callable::EnumVariantInit(e) => {
                e.fields = inputs;
                e.self_ = output;
            }
        }
        result
    }

    /// Returns a new [`Callable`] where all lifetime parameters in the
    /// output type (if present) are explicitly named.
    pub fn unelide_output_lifetimes(&self) -> Self {
        // Struct literals and enum variant constructors don't have lifetime elision.
        if matches!(
            self,
            Callable::StructLiteralInit(_) | Callable::EnumVariantInit(_)
        ) {
            return self.clone();
        }

        let Some(output) = self.output() else {
            return self.clone();
        };
        if !output.has_implicit_lifetime_parameters() {
            return self.clone();
        }

        let mut elided_output_lifetime: String = "elided".to_string();
        let mut inputs = self.inputs().to_vec();
        for input in inputs.iter_mut() {
            if input.type_.has_implicit_lifetime_parameters() {
                elided_output_lifetime = {
                    let mut named_lifetime_parameters = BTreeSet::new();
                    for input in self.input_types() {
                        named_lifetime_parameters.extend(input.named_lifetime_parameters());
                    }
                    named_lifetime_parameters.extend(output.named_lifetime_parameters());

                    if named_lifetime_parameters.contains("elided") {
                        let last = named_lifetime_parameters.last().unwrap();
                        format!("{last}_")
                    } else {
                        "elided".to_string()
                    }
                };

                input
                    .type_
                    .set_implicit_lifetimes(elided_output_lifetime.clone());
                break;
            }

            let named_params = input.type_.named_lifetime_parameters();
            if !named_params.is_empty() {
                elided_output_lifetime = named_params.first().unwrap().to_string();
                break;
            }
        }

        let mut output = output.clone();
        output.set_implicit_lifetimes(elided_output_lifetime);

        let mut result = self.clone();
        match &mut result {
            Callable::FreeFunction(f) => {
                f.header.output = Some(output);
                f.header.inputs = inputs;
            }
            Callable::InherentMethod(m) => {
                m.header.output = Some(output);
                m.header.inputs = inputs;
            }
            Callable::TraitMethod(m) => {
                m.header.output = Some(output);
                m.header.inputs = inputs;
            }
            // Early return above ensures we never reach here.
            Callable::StructLiteralInit(_) | Callable::EnumVariantInit(_) => unreachable!(),
        }
        result
    }

    /// Returns the indices of all input parameters that the output type
    /// borrows immutably from (i.e. not `&mut`).
    pub fn inputs_that_output_borrows_immutably_from(&self) -> Vec<usize> {
        let c = self.unelide_output_lifetimes();
        let Some(output) = c.output() else {
            return vec![];
        };

        let output_lifetime_parameters = output.named_lifetime_parameters();

        let mut borrowed_indexes = vec![];
        for (i, input) in c.input_types().enumerate() {
            let Type::Reference(ref_ty) = input else {
                continue;
            };
            if ref_ty.is_mutable {
                continue;
            }
            let Lifetime::Named(lifetime) = &ref_ty.lifetime else {
                continue;
            };
            if output_lifetime_parameters.contains(lifetime.as_str()) {
                borrowed_indexes.push(i)
            }
        }
        borrowed_indexes
    }

    /// Returns the indices of all input parameters share a lifetime parameter with the output
    pub fn inputs_with_lifetime_tied_with_output(&self) -> Vec<usize> {
        let c = self.unelide_output_lifetimes();
        let Some(output) = c.output() else {
            return vec![];
        };

        let output_lifetime_parameters = output.named_lifetime_parameters();

        let mut borrowed_indexes = vec![];
        for (i, input) in c.input_types().enumerate() {
            if input
                .named_lifetime_parameters()
                .intersection(&output_lifetime_parameters)
                .next()
                .is_some()
            {
                borrowed_indexes.push(i)
            }
        }
        borrowed_indexes
    }

    pub fn render_signature(&self, package_ids2names: &BiHashMap<PackageId, String>) -> String {
        let mut buffer = String::new();
        write!(&mut buffer, "{}", self).unwrap();
        write!(&mut buffer, "(").unwrap();
        let mut inputs = self.input_types().peekable();
        while let Some(input) = inputs.next() {
            write!(&mut buffer, "{}", input.render_type(package_ids2names)).unwrap();
            if inputs.peek().is_some() {
                write!(&mut buffer, ", ").unwrap();
            }
        }
        write!(&mut buffer, ")",).unwrap();
        if let Some(output) = self.output() {
            write!(&mut buffer, " -> {}", output.render_type(package_ids2names)).unwrap();
        }
        buffer
    }

    /// Returns the `extra_field2default_value` map if this is a `StructLiteralInit`, otherwise `None`.
    pub fn extra_field2default_value(&self) -> Option<&BTreeMap<String, String>> {
        match self {
            Callable::StructLiteralInit(s) => Some(&s.extra_field2default_value),
            _ => None,
        }
    }
}

impl std::fmt::Display for Callable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Callable::FreeFunction(inner) => write!(f, "{}", inner.path),
            Callable::InherentMethod(inner) => write!(f, "{}", inner.path),
            Callable::TraitMethod(inner) => write!(f, "{}", inner.path),
            Callable::StructLiteralInit(inner) => write!(f, "{}", inner.path),
            Callable::EnumVariantInit(inner) => write!(f, "{}", inner.path),
        }
    }
}

impl std::fmt::Debug for Callable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        write!(f, "(")?;
        let mut inputs = self.input_types().peekable();
        while let Some(input) = inputs.next() {
            write!(f, "{input:?}")?;
            if inputs.peek().is_some() {
                write!(f, ", ")?;
            }
        }
        write!(f, ")")?;
        if let Some(output) = self.output() {
            write!(f, " -> {output:?}")?;
        }
        Ok(())
    }
}
