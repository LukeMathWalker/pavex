pub use rustdoc_ir::*;

use crate::language::{FQPath, FQPathSegment};

pub(crate) trait PathTypeExt {
    fn resolved_path(&self) -> FQPath;
}

impl PathTypeExt for PathType {
    fn resolved_path(&self) -> FQPath {
        let mut segments = Vec::with_capacity(self.base_type.len());
        for segment in &self.base_type {
            segments.push(FQPathSegment {
                ident: segment.to_owned(),
                generic_arguments: vec![],
            });
        }
        FQPath {
            segments,
            qualified_self: None,
            package_id: self.package_id.clone(),
        }
    }
}
