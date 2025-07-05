use std::{collections::BTreeSet, path::Path};

use camino::Utf8PathBuf;

use crate::locator::find_tutorials_in_scope;

pub struct Tutorial {
    pub root_dir: Utf8PathBuf,
    pub patches: BTreeSet<Utf8PathBuf>,
}

impl Tutorial {
    pub fn new(root_dir: Utf8PathBuf) -> Result<Self, anyhow::Error> {
        let mut self_ = Self {
            root_dir,
            patches: Default::default(),
        };
        // Collect the patch files from the patch directory
        let patches_dir = self_.patches_dir();
        // Collect the paths of all .patch files in the patch directory
        if patches_dir.exists() {
            for entry in std::fs::read_dir(patches_dir)? {
                let entry = entry?;
                let path = Utf8PathBuf::from_path_buf(entry.path())
                    .map_err(|e| anyhow::anyhow!("Invalid UTF-8 path: {}", e.display()))?;
                if path.extension() == Some("patch") {
                    self_.patches.insert(path);
                } else {
                    anyhow::bail!("Unexpected file in `patches` directory: {path}")
                }
            }
        }
        Ok(self_)
    }

    pub fn name(&self) -> &str {
        self.root_dir.file_name().unwrap()
    }

    pub fn patches_dir(&self) -> Utf8PathBuf {
        self.root_dir.join("patches")
    }

    pub fn project_dir(&self) -> Utf8PathBuf {
        self.root_dir.join("project")
    }

    /// Check if the project folder has uncommitted or unstaged changes.
    pub fn is_clean(&self) -> Result<bool, anyhow::Error> {
        let output = std::process::Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(self.project_dir())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to check git status: {}", e))?;
        Ok(output.status.success() && output.stdout.is_empty())
    }

    /// Initialize the project folder as a git repository.
    pub fn git_init(&self) -> Result<(), anyhow::Error> {
        let project_dir = self.project_dir();
        let git_dir = project_dir.join(".git");
        if git_dir.exists() {
            anyhow::bail!(
                "The project folder is already initialized as a git repository! Run `pxh tutorial extract` before trying again."
            )
        }
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&project_dir)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to initialize git repository: {}", e))?;
        std::process::Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(&project_dir)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to add files to git: {}", e))?;
        std::process::Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("First commit")
            .current_dir(&project_dir)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to create initial commit: {}", e))?;
        Ok(())
    }

    pub fn patch_and_process(&mut self) -> Result<(), anyhow::Error> {
        for patch in &self.patches {
            eprintln!("Applying patch: {}", patch);
            let status = std::process::Command::new("git")
                .arg("am")
                .arg("--quiet")
                .arg(patch)
                .current_dir(self.project_dir())
                .stderr(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to invoke `git am`: {}", e))?;
            if !status.success() {
                anyhow::bail!("Failed to apply patch: git am exited with status {status}",);
            }
            eprintln!("Extracting snapshots");
            let status = std::process::Command::new("pxh")
                .arg("example")
                .arg("regenerate")
                .current_dir(self.project_dir())
                .stderr(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to invoke `pxh example generate`: {}", e))?;
            if !status.success() {
                anyhow::bail!(
                    "Failed to generate examples: `pxh example generate` exited with status {status}"
                );
            }
        }
        Ok(())
    }

    pub fn extract_patches(&mut self) -> Result<(), anyhow::Error> {
        eprintln!("Extracting patches for tutorial at {}", self.root_dir);
        if !self.is_clean()? {
            anyhow::bail!("There are uncommitted/unstaged changes. Can't extract.");
        }

        let root_commit_hash = {
            let output = std::process::Command::new("git")
                .arg("rev-list")
                .arg("--max-parents=0")
                .arg("HEAD")
                .current_dir(self.project_dir())
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to compute root commit hash: {}", e))?;
            if !output.status.success() {
                anyhow::bail!(
                    "Failed to compute root commit hash: `git rev-list` exited with status {}",
                    output.status
                );
            }
            String::from_utf8(output.stdout)
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in root commit hash: {}", e))?
                .trim()
                .to_string()
        };

        let output = std::process::Command::new("git")
            .arg("format-patch")
            .arg("--output-directory")
            .arg(self.patches_dir())
            .arg(&root_commit_hash)
            .current_dir(self.project_dir())
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to invoke `git format-patch`: {}", e))?;
        if !output.success() {
            anyhow::bail!(
                "Failed to extract patches: `git format-patch` exited with status {output}"
            );
        }

        eprintln!("Patches have been extracted. Reverting the `project` to the initial state.");

        // Checkout the first commit
        let status = std::process::Command::new("git")
            .arg("reset")
            .arg("--hard")
            .arg(&root_commit_hash)
            .current_dir(self.project_dir())
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to checkout the first commit: {}", e))?;
        if !status.success() {
            anyhow::bail!(
                "Failed to checkout the first commit: `git checkout` exited with status {status}"
            );
        }

        // Remove the .git folder
        let git_dir = self.project_dir().join(".git");
        if git_dir.exists() {
            std::fs::remove_dir_all(&git_dir)
                .map_err(|e| anyhow::anyhow!("Failed to remove .git directory {git_dir}: {e:?}"))?;
        }
        Ok(())
    }
}

pub fn hydrate_tutorials(cwd: &Path) -> Result<(), anyhow::Error> {
    let mut tutorials = collect_tutorials_in_scope(cwd)?;
    for tutorial in &mut tutorials {
        eprintln!("Hydrating tutorial at {}", tutorial.root_dir);
        tutorial.git_init()?;
        tutorial.patch_and_process()?;
    }
    Ok(())
}

pub fn extract_patches(cwd: &Path) -> Result<(), anyhow::Error> {
    let mut tutorials = collect_tutorials_in_scope(cwd)?;
    for tutorial in &mut tutorials {
        tutorial.extract_patches()?;
    }
    Ok(())
}

fn collect_tutorials_in_scope(cwd: &Path) -> Result<Vec<Tutorial>, anyhow::Error> {
    let tutorial_manifests = find_tutorials_in_scope(cwd)?;
    let mut tutorials = Vec::with_capacity(tutorial_manifests.len());
    for manifest_path in tutorial_manifests {
        let root_dir = manifest_path
            .parent()
            .expect("Tutorial manifest with no parent dir")
            .to_owned();
        tutorials.push(Tutorial::new(root_dir)?);
    }
    Ok(tutorials)
}
