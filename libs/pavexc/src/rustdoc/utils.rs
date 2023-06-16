// Ensure that crate names are in canonical form! Damn automated hyphen substitution!
pub fn normalize_crate_name(s: &str) -> String {
    s.replace('-', "_")
}
