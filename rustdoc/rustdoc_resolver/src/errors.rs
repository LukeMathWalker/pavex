//! Error types produced during type and callable resolution.

use std::sync::Arc;

use rustdoc_types::Type as RustdocType;

/// An error encountered while resolving a `rustdoc_types::Type` into a `rustdoc_ir::Type`.
#[derive(Debug)]
pub struct TypeResolutionError {
    pub ty: RustdocType,
    pub details: TypeResolutionErrorDetails,
}

impl std::fmt::Display for TypeResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Failed to resolve a type, {:?}.", self.ty)?;
        match &self.details {
            TypeResolutionErrorDetails::UnsupportedFnPointer(unsupported_fn_pointer) => {
                write!(
                    f,
                    "It uses a function pointer with inputs {:?} and output {:?}, which isn't currently supported.",
                    unsupported_fn_pointer.inputs, unsupported_fn_pointer.output
                )
            }
            TypeResolutionErrorDetails::UnsupportedReturnTypeNotation => {
                write!(
                    f,
                    "It uses return type notation, which isn't currently supported."
                )
            }
            TypeResolutionErrorDetails::UnsupportedTypeKind(unsupported_type_kind) => {
                write!(
                    f,
                    "It is a `{}`, which isn't currently supported.",
                    unsupported_type_kind.kind
                )
            }
            TypeResolutionErrorDetails::UnsupportedArrayLength(unsupported_array_length) => {
                write!(
                    f,
                    "It uses an array with length `{}`, which we can't evaluate at compile time.",
                    unsupported_array_length.len
                )
            }
            TypeResolutionErrorDetails::UnknownPrimitive(u) => {
                write!(f, "{u}")
            }
            TypeResolutionErrorDetails::GenericKindMismatch(mismatch) => {
                write!(
                    f,
                    "Expected a {} for the generic parameter `{}`, but found a {}.",
                    mismatch.expected_kind, mismatch.parameter_name, mismatch.found_kind
                )
            }
            TypeResolutionErrorDetails::ItemResolutionError(source) => {
                write!(f, "{source}")
            }
            TypeResolutionErrorDetails::TypePartResolutionError(source) => {
                write!(f, "Failed to resolve {}:\n{}", source.role, source.source)
            }
            TypeResolutionErrorDetails::AssociatedTypeResolutionError(e) => {
                write!(f, "{e}")
            }
        }
    }
}

impl std::error::Error for TypeResolutionError {}

/// The specific reason a type could not be resolved.
#[derive(Debug)]
pub enum TypeResolutionErrorDetails {
    UnsupportedFnPointer(UnsupportedFnPointer),
    UnsupportedReturnTypeNotation,
    UnsupportedTypeKind(UnsupportedTypeKind),
    UnsupportedArrayLength(UnsupportedArrayLength),
    UnknownPrimitive(rustdoc_ir::UnknownPrimitive),
    GenericKindMismatch(GenericKindMismatch),
    ItemResolutionError(anyhow::Error),
    TypePartResolutionError(Box<TypePartResolutionError>),
    /// We couldn't find a concrete `impl Trait for Type` block that defines the associated type.
    /// This can happen when the implementation is provided via a blanket impl (e.g.,
    /// `impl<T: Bound> Trait for T`), which we cannot resolve from rustdoc's JSON output.
    AssociatedTypeResolutionError(AssociatedTypeResolutionError),
}

/// A function pointer type was encountered, which is not yet supported.
#[derive(Debug)]
pub struct UnsupportedFnPointer {
    /// The input types, enclosed in parentheses.
    pub inputs: Vec<RustdocType>,
    /// The output type provided after the `->`, if present.
    pub output: Option<RustdocType>,
}

/// A sub-component of a type failed to resolve.
#[derive(Debug)]
pub struct TypePartResolutionError {
    pub role: String,
    pub source: TypeResolutionError,
}

/// A type kind that is not yet supported (e.g. `dyn Trait`, `impl Trait`).
#[derive(Debug)]
pub struct UnsupportedTypeKind {
    pub kind: &'static str,
}

/// An array length expression that cannot be evaluated at compile time.
#[derive(Debug)]
pub struct UnsupportedArrayLength {
    pub len: String,
}

/// We couldn't find a concrete `impl Trait for Type` block that defines the associated type.
#[derive(Debug)]
pub struct AssociatedTypeResolutionError {
    /// The associated type name (e.g., "Numeric").
    pub assoc_type_name: String,
    /// The trait path (e.g., ["enumflags2", "_internal", "RawBitFlags"]).
    pub trait_path: Vec<String>,
    /// The concrete self type that we resolved.
    pub self_type: rustdoc_ir::Type,
}

impl std::fmt::Display for AssociatedTypeResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trait_str = self.trait_path.join("::");
        write!(
            f,
            "Failed to resolve the associated type `{t}::{name}` for `{self_type:?}`. \
             No concrete `impl {t} for {self_type:?}` block was found that defines this \
             associated type. This can happen when the implementation is provided via a blanket \
             impl (e.g., `impl<T: Bound> {t} for T`), which we cannot resolve.",
            t = trait_str,
            name = self.assoc_type_name,
            self_type = self.self_type,
        )
    }
}

/// A generic argument did not match the expected kind (type vs lifetime vs const).
#[derive(Debug)]
pub struct GenericKindMismatch {
    pub parameter_name: String,
    pub expected_kind: &'static str,
    pub found_kind: &'static str,
}

/// An input parameter of a callable has a type that cannot be resolved.
#[derive(Debug, thiserror::Error, Clone)]
#[error("One of the input parameters for `{callable_path}` has a type that I can't handle.")]
pub struct InputParameterResolutionError {
    pub callable_path: String,
    pub callable_item: rustdoc_types::Item,
    pub parameter_type: RustdocType,
    pub parameter_index: usize,
    #[source]
    pub source: Arc<anyhow::Error>,
}

/// The `Self` type of a method could not be resolved.
#[derive(Debug, thiserror::Error, Clone)]
#[error("I can't handle the `Self` type for `{path}`.")]
pub struct SelfResolutionError {
    pub path: String,
    #[source]
    pub source: Arc<anyhow::Error>,
}

/// The return type of a callable could not be resolved.
#[derive(Debug, thiserror::Error, Clone)]
#[error("I don't know how to handle the type returned by `{callable_path}`.")]
pub struct OutputTypeResolutionError {
    pub callable_path: String,
    pub callable_item: rustdoc_types::Item,
    pub output_type: RustdocType,
    #[source]
    pub source: Arc<anyhow::Error>,
}

/// An error encountered while resolving a callable (function or method).
#[derive(thiserror::Error, Debug, Clone)]
pub enum CallableResolutionError {
    #[error(transparent)]
    SelfResolutionError(#[from] SelfResolutionError),
    #[error(transparent)]
    InputParameterResolutionError(#[from] InputParameterResolutionError),
    #[error(transparent)]
    OutputTypeResolutionError(#[from] OutputTypeResolutionError),
}
