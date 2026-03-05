/// Error type representing a path that could not be found in the rustdoc index.
#[derive(thiserror::Error, Debug)]
pub struct UnknownItemPath {
    pub path: Vec<String>,
}

impl std::fmt::Display for UnknownItemPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path.join("::").replace(' ', "");
        let krate = self.path.first().unwrap();
        write!(
            f,
            "I could not find '{path}' in the auto-generated documentation for '{krate}'."
        )
    }
}
