use std::env;
use std::path::PathBuf;

#[derive(Debug)]
/// Change the current working directory to the given path, and change it back
/// when this struct is dropped.
pub(crate) struct ScopedWorkingDirectory(PathBuf);

impl Default for ScopedWorkingDirectory {
    fn default() -> Self {
        Self(env::current_dir().unwrap())
    }
}

impl Drop for ScopedWorkingDirectory {
    fn drop(&mut self) {
        env::set_current_dir(&self.0).unwrap();
    }
}
