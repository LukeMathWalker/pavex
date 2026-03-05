use crate::language::Type;

pub struct PrebuiltType(Type);

impl PrebuiltType {
    pub(crate) fn new(ty: Type) -> Result<Self, PrebuiltTypeValidationError> {
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

impl From<PrebuiltType> for Type {
    fn from(input: PrebuiltType) -> Self {
        input.0
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum PrebuiltTypeValidationError {
    #[error("Prebuilt types can't have non-'static lifetime parameters.")]
    CannotHaveLifetimeParameters { ty: Type },
    #[error("Prebuilt types can't have unassigned generic type parameters.")]
    CannotHaveUnassignedGenericTypeParameters { ty: Type },
}
