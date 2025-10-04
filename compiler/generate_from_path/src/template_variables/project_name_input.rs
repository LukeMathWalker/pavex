use crate::template::LiquidObjectResource;
use liquid::ValueView;
use std::fmt::Display;

#[derive(Debug)]
pub struct ProjectNameInput(String);

impl From<&LiquidObjectResource> for ProjectNameInput {
    fn from(liquid_object: &LiquidObjectResource) -> Self {
        let name = liquid_object
            .get("project-name")
            .expect("`project-name` should be in liquid object");

        Self(name.as_scalar().to_kstr().into_string())
    }
}

impl AsRef<str> for ProjectNameInput {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Display for ProjectNameInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
