use crate::diagnostic::CompilerDiagnostic;
use indexmap::IndexSet;
use persist_if_changed::{has_changed_file2buffer, persist_if_changed};
use std::path::{Path, PathBuf};

pub struct AppWriter {
    pub mode: WriterMode,
}

pub enum WriterMode {
    Update,
    /// If in check mode, `AppWriter` doesn't update the contents stored
    /// on disk.
    /// It only keeps track of which files have changed.
    CheckOnly {
        outdated: IndexSet<PathBuf>,
    },
}

impl AppWriter {
    pub fn check_mode() -> Self {
        Self {
            mode: WriterMode::CheckOnly {
                outdated: Default::default(),
            },
        }
    }

    pub fn update_mode() -> Self {
        Self {
            mode: WriterMode::Update,
        }
    }

    pub fn persist_if_changed(&mut self, path: &Path, content: &[u8]) -> Result<(), anyhow::Error> {
        match &mut self.mode {
            WriterMode::CheckOnly { outdated } => {
                if has_changed_file2buffer(path, content)? {
                    outdated.insert(path.to_path_buf());
                }
            }
            _ => {
                persist_if_changed(path, content)?;
            }
        }
        Ok(())
    }

    pub fn verify(&self) -> Result<(), Vec<miette::Error>> {
        let WriterMode::CheckOnly { outdated } = &self.mode else {
            return Ok(());
        };
        if outdated.is_empty() {
            return Ok(());
        }
        let mut errors = vec![];
        for o in outdated {
            // TODO: print a diff showing what's wrong.
            let e = anyhow::anyhow!("`{}` is not up-to-date.", o.display());
            let diagnostic = CompilerDiagnostic::builder(e)
                .help(
                    "Regenerate the project (e.g. by running `cargo px check`) to fix the issue."
                        .into(),
                )
                .build();
            errors.push(diagnostic.into());
        }
        Err(errors)
    }
}
