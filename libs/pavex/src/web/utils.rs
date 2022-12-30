use crate::language::ResolvedType;

/// Returns `true` if `t` is a `Result` type.
pub(crate) fn is_result(t: &ResolvedType) -> bool {
    t.base_type == ["core", "result", "Result"]
        || t.base_type == ["core", "prelude", "rust_2015", "v1", "Result"]
        || t.base_type == ["core", "prelude", "rust_2018", "v1", "Result"]
        || t.base_type == ["core", "prelude", "rust_2021", "v1", "Result"]
}
