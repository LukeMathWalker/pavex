use crate::language::ResolvedType;

pub struct StateInput(ResolvedType);

impl StateInput {
    pub fn new(ty: ResolvedType) -> Result<Self, StateInputValidationError> {
        if ty.has_implicit_lifetime_parameters() || !ty.named_lifetime_parameters().is_empty() {
            return Err(StateInputValidationError::CannotHaveLifetimeParameters);
        }
        if !ty.unassigned_generic_type_parameters().is_empty() {
            return Err(StateInputValidationError::CannotHaveUnassignedGenericTypeParameters);
        }
        Ok(Self(ty))
    }
}

impl From<StateInput> for ResolvedType {
    fn from(input: StateInput) -> Self {
        input.0
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum StateInputValidationError {
    #[error("Types that are used as inputs to build the application state can't have non-'static lifetime parameters.")]
    CannotHaveLifetimeParameters,
    #[error("Types that are used as inputs to build the application state can't have unassigned generic type parameters.")]
    CannotHaveUnassignedGenericTypeParameters,
}
