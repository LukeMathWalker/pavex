pub use rustdoc_ir::*;

use crate::language::{FQPath, FQPathSegment, StructLiteralPath};

pub(crate) trait PathTypeExt {
    fn resolved_path(&self) -> FQPath;
    fn callable_struct_literal_path(&self) -> StructLiteralPath;
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

    fn callable_struct_literal_path(&self) -> StructLiteralPath {
        // base_type is [crate_name, module1, ..., TypeName]
        let crate_name = self.base_type.first().cloned().unwrap_or_default();
        let type_name = self.base_type.last().cloned().unwrap_or_default();
        let module_path = if self.base_type.len() > 2 {
            self.base_type[1..self.base_type.len() - 1].to_vec()
        } else {
            vec![]
        };
        StructLiteralPath {
            package_id: self.package_id.clone(),
            crate_name,
            module_path,
            type_name,
            type_generics: vec![],
        }
    }
}
