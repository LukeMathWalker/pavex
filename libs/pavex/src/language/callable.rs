use std::fmt::Formatter;
use std::fmt::Write;

use bimap::BiHashMap;
use guppy::PackageId;

use crate::language::{ResolvedPath, ResolvedType};

#[derive(Clone, Hash, Eq, PartialEq)]
pub(crate) struct Callable {
    pub is_async: bool,
    pub output: ResolvedType,
    pub path: ResolvedPath,
    pub inputs: Vec<ResolvedType>,
}

impl Callable {
    pub fn render_signature(&self, package_ids2names: &BiHashMap<&'_ PackageId, String>) -> String {
        let mut buffer = String::new();
        write!(&mut buffer, "{}", self.path).unwrap();
        write!(&mut buffer, "(").unwrap();
        let mut inputs = self.inputs.iter().peekable();
        while let Some(input) = inputs.next() {
            write!(&mut buffer, "{}", input.render_type(package_ids2names)).unwrap();
            if inputs.peek().is_some() {
                write!(&mut buffer, ", ").unwrap();
            }
        }
        write!(
            &mut buffer,
            ") -> {}",
            self.output.render_type(package_ids2names)
        )
        .unwrap();
        buffer
    }
}

impl std::fmt::Debug for Callable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)?;
        write!(f, "(")?;
        let mut inputs = self.inputs.iter().peekable();
        while let Some(input) = inputs.next() {
            write!(f, "{:?}", input)?;
            if inputs.peek().is_some() {
                write!(f, ", ")?;
            }
        }
        write!(f, ") -> {:?}", self.output)?;
        Ok(())
    }
}
