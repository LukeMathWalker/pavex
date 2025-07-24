use crate::{
    changeset::print_changeset,
    locator::find_examples_in_scope,
    snippets::{
        ExternalSnippetSpec, SnippetName, SourceRange, extract_embedded_snippets,
        extract_external_snippet,
    },
};
use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use globwalk::GlobWalkerBuilder;
use std::{
    collections::BTreeMap,
    io::Write as _,
    path::Path,
    process::{Command, ExitStatus},
};

pub struct Example {
    pub root_dir: Utf8PathBuf,
    pub manifest: ExampleManifest,
    pub embedded_snippets: BTreeMap<SnippetName, String>,
    pub external_snippets: BTreeMap<SnippetName, String>,
}

impl Example {
    pub fn new(root_dir: Utf8PathBuf, manifest: ExampleManifest) -> Self {
        Example {
            root_dir,
            manifest,
            embedded_snippets: BTreeMap::new(),
            external_snippets: BTreeMap::new(),
        }
    }

    pub fn snippets_base_dir(&self) -> &Utf8PathBuf {
        self.manifest
            .snippets_base_dir
            .as_ref()
            .unwrap_or(&self.root_dir)
    }

    pub fn extract_embedded_snippets(&mut self) -> Result<(), anyhow::Error> {
        let paths: Vec<Utf8PathBuf> =
            GlobWalkerBuilder::from_patterns(&self.root_dir, &["**/*.rs", "**/*.toml"])
                .build()?
                .filter_map(|entry| entry.ok())
                .map(|entry| Utf8PathBuf::from_path_buf(entry.into_path()).ok())
                .flatten()
                .collect();
        for path in paths {
            for snippet in extract_embedded_snippets(&path)? {
                let title = self.manifest.use_path_as_title.then(|| {
                    pathdiff::diff_paths(path.as_std_path(), self.root_dir.as_std_path())
                        .expect("Failed to diff paths")
                        .to_str()
                        .unwrap()
                        .to_owned()
                });
                let rendered = snippet.render(&path, title);
                if self
                    .embedded_snippets
                    .insert(snippet.name.clone(), rendered)
                    .is_some()
                {
                    anyhow::bail!(
                        "There are multiple embedded snippets named '{}'",
                        snippet.name
                    );
                }
            }
        }
        Ok(())
    }

    pub fn extract_external_snippets(&mut self) -> Result<(), anyhow::Error> {
        for spec in &self.manifest.snippets {
            let spec = ExternalSnippetSpec {
                name: SnippetName::new(spec.name.clone())?,
                source_path: spec.source_path.clone(),
                ranges: spec
                    .ranges
                    .iter()
                    .map(|r| r.parse())
                    .collect::<Result<Vec<SourceRange>, _>>()?,
                hl_lines: spec.hl_lines.clone(),
            };
            let title = self
                .manifest
                .use_path_as_title
                .then(|| spec.source_path.as_str());
            let snippet = extract_external_snippet(&self.root_dir, title, &spec)?;
            if self
                .external_snippets
                .insert(spec.name.clone(), snippet)
                .is_some()
            {
                anyhow::bail!(
                    "There are multiple external snippets named '{}'",
                    &spec.name
                );
            }
        }
        Ok(())
    }

    pub fn compile_example(&self, verify: bool) -> Result<(), anyhow::Error> {
        eprintln!("Compiling example...");
        let (stderr_reader, stderr_writer) = std::io::pipe()?;
        let mut stderr = String::new();
        let save_stderr = self.manifest.compilation_output_filename.is_some()
            || self.manifest.compilation == CompilationOutcome::Failure;

        let status = std::thread::scope(|scope| {
            scope.spawn(|| {
                use std::io::BufRead as _;

                let mut reader = std::io::BufReader::new(stderr_reader);
                let mut buffer = String::new();
                loop {
                    let mut line = String::new();
                    let bytes = reader.read_line(&mut line).unwrap_or(0);
                    if bytes == 0 {
                        break;
                    }
                    // Forward to parent stderr
                    eprint!("{}", line);
                    buffer.push_str(&line);
                }

                stderr = buffer;
            });

            let mut cmd = Command::new("cargo");

            cmd.arg("px")
                .arg(if verify { "verify-freshness" } else { "check" });

            if save_stderr {
                // Avoid stray output, otherwise it'll be captured in the snippet.
                cmd.arg("--quiet")
                    .env("PAVEXC_COLOR", "always")
                    .env("PAVEXC_QUIET", "true");
            };

            let status = cmd
                .env("PAVEX_PAVEXC", "pavexc")
                .env("PAVEX_TTY_WIDTH".to_string(), "70".to_string())
                .current_dir(self.root_dir.join("server_sdk"))
                .stderr(stderr_writer)
                .status()
                .context("Failed to invoke `cargo px check`")?;

            Result::<ExitStatus, anyhow::Error>::Ok(status)
        })?;

        match self.manifest.compilation {
            CompilationOutcome::Success => {
                if !status.success() {
                    eprintln!("Expected example to compile successfully, but it failed.\n{stderr}",);
                    anyhow::bail!("Failed to compile example");
                }
            }
            CompilationOutcome::Failure => {
                if status.success() {
                    eprintln!("Expected example to fail to compile, but it succeeded!");
                    anyhow::bail!("Unexpected success compiling example");
                }

                // Denoise the output for documentation purposes.
                stderr = stderr
                    .lines()
                    .filter(|l| {
                        l != &"The invocation of `pavex [...] generate [...]` exited with a non-zero status code: 1" &&
                        !l.starts_with("error: Failed to run `bp`, the code generator for") &&
                        !l.starts_with("[1m[36mnote[0m[1m:[0m Rerun with `PAVEX_DEBUG=true` to display more error details")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }

        if save_stderr {
            let mut options = fs_err::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            let mut file = options.open(
                self.snippets_base_dir()
                    .join(format!("{}.snap", self.stderr_snippet_name())),
            )?;
            file.write_all(stderr.as_bytes())?;
        }

        Ok(())
    }

    pub fn check_or_save_external_snippets(&mut self, verify: bool) -> Result<(), anyhow::Error> {
        let mut has_failed = false;
        for (name, snippet) in &self.external_snippets {
            if let Err(e) = check_or_save_snippet(self.snippets_base_dir(), &name, &snippet, verify)
            {
                eprintln!("{e}");
                has_failed = true;
            }
        }

        if has_failed {
            anyhow::bail!("One or more external snippets didn't match expectations");
        } else {
            Ok(())
        }
    }

    pub fn check_or_save_embedded_snippets(&mut self, verify: bool) -> Result<(), anyhow::Error> {
        let mut has_failed = false;
        for (name, snippet) in &self.embedded_snippets {
            if let Err(e) = check_or_save_snippet(self.snippets_base_dir(), &name, &snippet, verify)
            {
                eprintln!("{e}");
                has_failed = true;
            }
        }

        if has_failed {
            anyhow::bail!("One or more embedded snippets didn't match expectations");
        } else {
            Ok(())
        }
    }

    pub fn stderr_snippet_name(&self) -> &str {
        self.manifest
            .compilation_output_filename
            .as_deref()
            .unwrap_or("stderr")
    }

    /// Collect all `*.snap` files at the root of the example directory.
    /// If their name doesn't match one of the extracted snippets (either embedded or external),
    /// they are considered orphaned and will be deleted.
    pub fn prune_orphan_snippets(&mut self) -> Result<(), anyhow::Error> {
        if !self.manifest.prune_orphaned_snippets {
            eprintln!(
                "Skipping pruning of orphaned snippets, as requested in the example manifest."
            );
            return Ok(());
        }

        let snap_files: Vec<Utf8PathBuf> =
            GlobWalkerBuilder::from_patterns(&self.snippets_base_dir(), &["*.snap"])
                .build()?
                .filter_map(|entry| entry.ok())
                .map(|entry| Utf8PathBuf::from_path_buf(entry.into_path()).ok())
                .flatten()
                .collect();

        let mut valid_snippet_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        valid_snippet_names.extend(self.embedded_snippets.keys().map(|s| s.as_str().to_owned()));
        valid_snippet_names.extend(self.external_snippets.keys().map(|s| s.as_str().to_owned()));
        valid_snippet_names.insert(self.stderr_snippet_name().into());

        for snap_file in snap_files {
            if let Some(file_name) = snap_file.file_stem()
                && !valid_snippet_names.contains(file_name)
            {
                eprintln!("Deleting stale snippet file: {}", snap_file);
                fs_err::remove_file(&snap_file)?;
            }
        }
        Ok(())
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ExampleManifest {
    #[serde(default)]
    /// Snippets to be extracted from files in the example folder.
    ///
    /// Most snippets will be embedded in the source file itself, via comments, but
    /// you can't embed comments everywhere (e.g. not in generated files).
    snippets: Vec<ExternalSnippet>,
    /// By default, snippets are saved next to the manifest file.
    ///
    /// You can specify a different base directory for snippets using the `snippets_base_dir` field.
    #[serde(default)]
    snippets_base_dir: Option<Utf8PathBuf>,
    #[serde(default)]
    /// Whether to use the (relative) path to the file from which the snippet is extracted as the snippet
    /// title.
    use_path_as_title: bool,
    #[serde(default = "default_prune_orphaned_snippets")]
    /// Whether to delete orphaned snippet files.
    prune_orphaned_snippets: bool,
    #[serde(default)]
    /// Whether the example is expected to compile successfully or not.
    compilation: CompilationOutcome,
    #[serde(default)]
    /// Save the `stderr` output coming from compilation to a file with the given name.
    ///
    /// If unspecified, the output will be saved to `stderr.snap` if the example is
    /// expected to fail to compile.
    compilation_output_filename: Option<String>,
}

fn default_prune_orphaned_snippets() -> bool {
    true
}

#[derive(serde::Deserialize, Default, Debug, PartialEq, Eq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CompilationOutcome {
    #[default]
    Success,
    Failure,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalSnippet {
    /// The name of this snippet.
    ///
    /// The snippet will be saved as `<name>.snap`, in the root of the example/tutorial folder.
    name: String,
    /// The path to the source file we want to extract the snippet from.
    ///
    /// It's expected to be relative to the root of the example/tutorial folder.
    source_path: Utf8PathBuf,
    /// The ranges of lines we want to extract from the source file.
    ranges: Vec<String>,
    /// Which lines should be highlighted in the snippet.
    /// The line numbers are relative to the start of the snippet, **not** to the
    /// line numbers in the original source file.
    #[serde(default)]
    hl_lines: Vec<usize>,
}

pub fn process_examples(cwd: &Path, verify: bool) -> Result<(), anyhow::Error> {
    let mut examples = collect_examples_in_scope(cwd)?;

    // First we extract embedded snippets.
    {
        let mut has_errors = false;
        for example in examples.iter_mut() {
            eprintln!("Processing embedded snippets from {}", example.root_dir);
            example.extract_embedded_snippets()?;
            if let Err(e) = example.check_or_save_embedded_snippets(verify) {
                has_errors = true;
                eprintln!("Error: {e:?}");
            }
        }

        if has_errors {
            anyhow::bail!("Errors occurred while processing embedded snippets")
        }
    }

    // Then try to compile.
    {
        let mut has_errors = false;
        for example in examples.iter_mut() {
            if let Err(e) = example.compile_example(verify) {
                has_errors = true;
                eprintln!("Compilation mismatch: {e:?}");
            }
        }
        if has_errors {
            anyhow::bail!("Errors occurred while compiling examples")
        }
    }

    // Finally, we extract external snippets.
    // We do the extraction after compilation since external snippets are primarily used
    // to extract snippets from code-generated files.
    {
        let mut has_errors = false;
        for example in examples.iter_mut() {
            eprintln!("Processing external snippets from {}", example.root_dir);
            example.extract_external_snippets()?;
            if let Err(e) = example.check_or_save_external_snippets(verify) {
                has_errors = true;
                eprintln!("Error: {e:?}");
            }
        }
        if has_errors {
            anyhow::bail!("Errors occurred while processing embedded snippets")
        }
    }

    // Last but not least, remove orphaned snap files.
    for example in examples.iter_mut() {
        example.prune_orphan_snippets()?;
    }

    Ok(())
}

fn collect_examples_in_scope(cwd: &Path) -> Result<Vec<Example>, anyhow::Error> {
    let example_manifests = find_examples_in_scope(cwd)?;
    let mut examples = Vec::with_capacity(example_manifests.len());
    for manifest_path in example_manifests {
        let root_dir = manifest_path
            .parent()
            .expect("Example manifest with no parent dir")
            .to_owned();
        let manifest = fs_err::read_to_string(manifest_path)?;
        let manifest: ExampleManifest = serde_yaml::from_str(&manifest)?;
        examples.push(Example::new(root_dir, manifest));
    }
    Ok(examples)
}

fn check_or_save_snippet(
    example_dir: &Utf8Path,
    name: &SnippetName,
    snippet: &str,
    verify: bool,
) -> Result<(), anyhow::Error> {
    let snippet_path = example_dir.join(format!("{}.snap", name));
    if verify {
        let expected = fs_err::read_to_string(&snippet_path)
            .context("Failed to read existing snippet to verify the extracted one. Perhaps you need to generate it first?")?;
        if expected != snippet {
            let mut err_msg = format!(
                "Expected snippet at {} did not match the extracted snippet.\n",
                snippet_path
            );
            print_changeset(&expected, &snippet, &mut err_msg)?;
            anyhow::bail!("{err_msg}");
        }
    } else {
        let mut options = fs_err::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        let mut file = options
            .open(&snippet_path)
            .context("Failed to open/create expectation file")?;
        file.write_all(snippet.as_bytes())
            .context("Failed to write to expectation file")?;
    }
    Ok(())
}
