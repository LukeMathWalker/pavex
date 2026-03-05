use std::fmt::{Debug, Display, Formatter};

/// A named lifetime (without leading `'`). Cannot be `"_"` or `"static"`.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct NamedLifetime(String);

impl NamedLifetime {
    pub fn new(name: impl Into<String>) -> Self {
        let mut name = name.into();
        if let Some(stripped) = name.strip_prefix('\'') {
            name = stripped.to_owned();
        }
        assert!(
            name != "_",
            "Use GenericLifetimeParameter::Inferred for inferred lifetimes ('_')"
        );
        assert!(
            name != "static",
            "Use GenericLifetimeParameter::Static for 'static"
        );
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for NamedLifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for NamedLifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
