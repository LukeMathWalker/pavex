use std::fmt::Formatter;
use std::fmt::Write;

use bimap::BiHashMap;
use guppy::PackageId;

use crate::language::{ResolvedPath, ResolvedType};

#[derive(Clone, Hash, Eq, PartialEq)]
pub(crate) struct Callable {
    pub output_fq_path: ResolvedType,
    pub callable_fq_path: ResolvedPath,
    pub inputs: Vec<ResolvedType>,
}

impl Callable {
    pub fn render_signature(&self, package_ids2names: &BiHashMap<&'_ PackageId, String>) -> String {
        let mut buffer = String::new();
        write!(&mut buffer, "{}", self.callable_fq_path).unwrap();
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
            self.output_fq_path.render_type(package_ids2names)
        )
        .unwrap();
        buffer
    }
}

impl std::fmt::Debug for Callable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.callable_fq_path)?;
        write!(f, "(")?;
        let mut inputs = self.inputs.iter().peekable();
        while let Some(input) = inputs.next() {
            write!(f, "{:?}", input)?;
            if inputs.peek().is_some() {
                write!(f, ", ")?;
            }
        }
        write!(f, ") -> {:?}", self.output_fq_path)?;
        Ok(())
    }
}
