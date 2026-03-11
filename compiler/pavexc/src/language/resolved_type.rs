pub use rustdoc_ir::*;

pub(crate) trait PathTypeExt {
    fn callable_struct_literal_path(&self) -> StructLiteralPath;
}

impl PathTypeExt for PathType {
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
