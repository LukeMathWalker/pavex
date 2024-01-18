use super::ProjectNameInput;
use heck::ToKebabCase;
use std::fmt::Display;

#[derive(Debug)]
pub struct ProjectName(String);

impl From<&ProjectNameInput> for ProjectName {
    fn from(project_name_input: &ProjectNameInput) -> Self {
        Self(project_name_input.as_ref().to_kebab_case())
    }
}

impl AsRef<str> for ProjectName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Display for ProjectName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
