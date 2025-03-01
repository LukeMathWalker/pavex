use crate::language::ResolvedType;

/// How to handle missing values for a config type.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum DefaultStrategy {
    /// `Default::default` will be invoked to generate default values
    /// if the user hasn't provided one.
    DefaultIfMissing,
    /// The user *must* provide a value for this config type.
    Required,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ConfigType {
    ty: ResolvedType,
    key: String,
}

impl ConfigType {
    pub(crate) fn new(ty: ResolvedType, key: String) -> Result<Self, ConfigTypeValidationError> {
        use ConfigTypeValidationError::*;

        if let Err(e) = syn::parse_str::<syn::Ident>(&key) {
            return Err(KeyMustAValidRustIdentifier { key, source: e });
        }
        if !ty.lifetime_parameters().is_empty() {
            return Err(CannotHaveAnyLifetimeParameters { ty });
        }
        if !ty.unassigned_generic_type_parameters().is_empty() {
            return Err(CannotHaveUnassignedGenericTypeParameters { ty });
        }
        Ok(Self { ty, key })
    }

    pub(crate) fn ty(&self) -> &ResolvedType {
        &self.ty
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}

impl From<ConfigType> for ResolvedType {
    fn from(input: ConfigType) -> Self {
        input.ty
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ConfigTypeValidationError {
    #[error("Configuration types can't have any lifetime parameter.")]
    CannotHaveAnyLifetimeParameters { ty: ResolvedType },
    #[error("Configuration types can't have unassigned generic type parameters.")]
    CannotHaveUnassignedGenericTypeParameters { ty: ResolvedType },
    #[error("The key for a configuration type must be a valid Rust identifier.")]
    KeyMustAValidRustIdentifier { key: String, source: syn::Error },
}
