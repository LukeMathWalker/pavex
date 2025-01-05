use std::borrow::Cow;

use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::component::CannotTakeMutReferenceError;
use indexmap::IndexSet;

use crate::compiler::computation::{Computation, MatchResult};
use crate::language::ResolvedType;

/// Build a new instance of a type by performing a computation.
///
/// The constructor can take zero or more arguments as inputs.
/// It must return a non-unit output type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Constructor<'a>(pub(crate) Computation<'a>);

impl<'a> Constructor<'a> {
    pub fn new(
        c: Computation<'a>,
        pavex_error: &ResolvedType,
        pavex_response: &ResolvedType,
        framework_item_db: &FrameworkItemDb,
    ) -> Result<Self, ConstructorValidationError> {
        if c.output_type().is_none() {
            return Err(ConstructorValidationError::CannotReturnTheUnitType);
        }
        let mut output_type = c.output_type().unwrap().to_owned();

        // If the constructor is fallible, we make sure that it returns a non-unit type on
        // the happy path.
        if output_type.is_result() {
            let m = MatchResult::match_result(&output_type);
            output_type = m.ok.output;
            if output_type == ResolvedType::UNIT_TYPE {
                return Err(ConstructorValidationError::CannotFalliblyReturnTheUnitType);
            }
        }

        if let Computation::Callable(c) = &c {
            CannotTakeMutReferenceError::check_callable(c.as_ref())?;
        }

        // You can't construct `pavex::Error` or `&pavex::Error`.
        if &output_type == pavex_error {
            return Err(ConstructorValidationError::CannotConstructPavexError);
        }
        if let ResolvedType::Reference(ref_type) = &output_type {
            if ref_type.inner.as_ref() == pavex_error {
                return Err(ConstructorValidationError::CannotConstructPavexError);
            }
        }

        // You can't construct `pavex::Response` or `&pavex::Response`.
        if &output_type == pavex_response {
            return Err(ConstructorValidationError::CannotConstructPavexResponse);
        }
        if let ResolvedType::Reference(ref_type) = &output_type {
            if ref_type.inner.as_ref() == pavex_response {
                return Err(ConstructorValidationError::CannotConstructPavexResponse);
            }
        }

        for (_, framework_primitive_type) in framework_item_db.iter() {
            if &output_type == framework_primitive_type {
                return Err(
                    ConstructorValidationError::CannotConstructFrameworkPrimitive {
                        primitive_type: framework_primitive_type.to_owned(),
                    },
                );
            }
            if let ResolvedType::Reference(ref_type) = &output_type {
                if ref_type.inner.as_ref() == framework_primitive_type {
                    return Err(
                        ConstructorValidationError::CannotConstructFrameworkPrimitive {
                            primitive_type: framework_primitive_type.to_owned(),
                        },
                    );
                }
            }
        }

        if let ResolvedType::Generic(g) = output_type {
            return Err(ConstructorValidationError::NakedGenericOutputType {
                naked_parameter: g.name,
            });
        }

        let output_unassigned_generic_parameters = output_type.unassigned_generic_type_parameters();
        let mut free_parameters = IndexSet::new();
        for input in c.input_types().as_ref() {
            free_parameters.extend(
                input
                    .unassigned_generic_type_parameters()
                    .difference(&output_unassigned_generic_parameters)
                    .cloned(),
            );
        }
        if !free_parameters.is_empty() {
            return Err(
                ConstructorValidationError::UnderconstrainedGenericParameters {
                    parameters: free_parameters,
                },
            );
        }

        Ok(Constructor(c))
    }
}

impl<'a> From<Constructor<'a>> for Computation<'a> {
    fn from(value: Constructor<'a>) -> Self {
        value.0
    }
}

impl Constructor<'_> {
    /// The type returned by the constructor.
    pub fn output_type(&self) -> &ResolvedType {
        self.0.output_type().unwrap()
    }

    /// The inputs types used by this constructor.
    pub fn input_types(&self) -> Cow<[ResolvedType]> {
        self.0.input_types()
    }

    /// Returns `true` if the constructor is fallible—that is, if it returns a `Result`.
    pub fn is_fallible(&self) -> bool {
        self.output_type().is_result()
    }

    pub fn into_owned(self) -> Constructor<'static> {
        Constructor(self.0.into_owned())
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum ConstructorValidationError {
    #[error("All constructors must return *something*.\nThis constructor doesn't: it returns the unit type, `()`.")]
    CannotReturnTheUnitType,
    #[error("All fallible constructors must return *something* when successful.\nThis fallible constructor doesn't: it returns the unit type when successful, `Ok(())`.")]
    CannotFalliblyReturnTheUnitType,
    #[error("You can't register a constructor for `pavex::Error`.\n`pavex::Error` can only be used as the error type of your fallible components.")]
    CannotConstructPavexError,
    #[error("You can't register a constructor for `pavex::Response`.\nYou can only return a response from request handlers, middlewares or error handlers.")]
    CannotConstructPavexResponse,
    #[error("You can't register a constructor for `{primitive_type:?}`.\n\
        `{primitive_type:?}` is a framework primitive, you can't override the way it's built by Pavex.")]
    CannotConstructFrameworkPrimitive { primitive_type: ResolvedType },
    #[error("Input parameters for a constructor can't have any *unassigned* generic type parameters that appear exclusively in its input parameters.")]
    UnderconstrainedGenericParameters { parameters: IndexSet<String> },
    #[error("The output type of a constructor can't be a naked generic parameters (i.e. `T`).\n\
        Pavex ignores trait bounds when looking at generic parameters, therefore a constructor \
        that returns a generic `T` is a constructor that can build **any** type - which is unlikely \
        to be the case.")]
    NakedGenericOutputType { naked_parameter: String },
    #[error(transparent)]
    CannotTakeAMutableReferenceAsInput(#[from] CannotTakeMutReferenceError),
}
