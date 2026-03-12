pub use rustdoc_ir::*;

pub(crate) fn get_ok_variant(t: &Type) -> &Type {
    debug_assert!(t.is_result());
    let Type::Path(t) = t else {
        unreachable!();
    };
    let GenericArgument::TypeParameter(t) = &t.generic_arguments[0] else {
        unreachable!()
    };
    t
}

pub(crate) fn get_err_variant(t: &Type) -> &Type {
    debug_assert!(t.is_result());
    let Type::Path(t) = t else {
        unreachable!();
    };
    let GenericArgument::TypeParameter(t) = &t.generic_arguments[1] else {
        unreachable!()
    };
    t
}

/// A generator of unique lifetime names.
#[derive(Debug, Clone)]
pub struct LifetimeGenerator {
    next: usize,
}

impl LifetimeGenerator {
    const ALPHABET: [char; 26] = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// Generates a new lifetime name.
    pub fn next(&mut self) -> String {
        let next = self.next;
        self.next += 1;
        let round = next / Self::ALPHABET.len();
        let letter = Self::ALPHABET[next % Self::ALPHABET.len()];
        if round == 0 {
            format!("{letter}")
        } else {
            format!("{letter}{round}")
        }
    }
}

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
