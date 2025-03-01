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
    key: ConfigKey,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
/// A configuration key.
///
/// It must start with a letter.
/// It can only contain letters, numbers, and underscores.
pub struct ConfigKey(String);

impl std::fmt::Display for ConfigKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ConfigKey {
    fn new(key: String) -> Result<Self, ConfigKeyValidationError> {
        let first = key
            .chars()
            .next()
            .ok_or(ConfigKeyValidationError::CannotBeEmpty)?;
        if !first.is_ascii_alphabetic() {
            return Err(ConfigKeyValidationError::InvalidStart { key, first });
        }
        for c in key.chars().skip(1) {
            if !c.is_ascii_alphanumeric() && c != '_' {
                return Err(ConfigKeyValidationError::InvalidChar { key, invalid: c });
            }
        }
        Ok(Self(key))
    }

    /// Convert the key into a valid Rust identifier.
    ///
    /// Infallible, as the key is guaranteed to be a valid Rust identifier.
    pub fn ident(&self) -> syn::Ident {
        syn::parse_str(&self.0).unwrap()
    }
}

impl ConfigType {
    pub(crate) fn new(ty: ResolvedType, key: String) -> Result<Self, ConfigTypeValidationError> {
        use ConfigTypeValidationError::*;

        let key = ConfigKey::new(key)?;
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

    pub(crate) fn key(&self) -> &ConfigKey {
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
    #[error(transparent)]
    InvalidKey(#[from] ConfigKeyValidationError),
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ConfigKeyValidationError {
    #[error("Configuration keys can't be empty.")]
    CannotBeEmpty,
    #[error(
        "Configuration keys must begin with a letter.\n\
        `{key}` starts with `{first}` which is not a letter."
    )]
    InvalidStart { key: String, first: char },
    #[error(
        "Configuration keys can only contain letters, digits, and underscores.\n\
        `{key}` contains `{invalid}` which is not a letter, digit, or underscore."
    )]
    InvalidChar { key: String, invalid: char },
}
