use crate::language::ResolvedType;

pub struct PrebuiltType(ResolvedType);

impl PrebuiltType {
    pub(crate) fn new(ty: ResolvedType) -> Result<Self, PrebuiltTypeValidationError> {
        if ty.has_implicit_lifetime_parameters() || !ty.named_lifetime_parameters().is_empty() {
            return Err(PrebuiltTypeValidationError::CannotHaveLifetimeParameters { ty });
        }
        if !ty.unassigned_generic_type_parameters().is_empty() {
            return Err(
                PrebuiltTypeValidationError::CannotHaveUnassignedGenericTypeParameters { ty },
            );
        }
        Ok(Self(ty))
    }
}

impl From<PrebuiltType> for ResolvedType {
    fn from(input: PrebuiltType) -> Self {
        input.0
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum PrebuiltTypeValidationError {
    #[error("Prebuilt types can't have non-'static lifetime parameters.")]
    CannotHaveLifetimeParameters { ty: ResolvedType },
    #[error("Prebuilt types can't have unassigned generic type parameters.")]
    CannotHaveUnassignedGenericTypeParameters { ty: ResolvedType },
}
