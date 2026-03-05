/// Adapter that reports rustdoc progress via Pavex's CLI shell.
pub(super) struct ShellProgress;

impl rustdoc_processor::ComputeProgress for ShellProgress {
    fn before_computing(
        &self,
        package_graph: &guppy::graph::PackageGraph,
        package_ids: &[guppy::PackageId],
    ) {
        if let Some(shell) = pavex_cli_shell::SHELL.get()
            && let Ok(mut shell) = shell.lock()
        {
            for package_id in package_ids {
                let Ok(meta) = package_graph.metadata(package_id) else {
                    continue;
                };
                let _ = shell.status("Documenting", format!("{}@{}", meta.name(), meta.version()));
            }
        }
    }

    fn after_computed(
        &self,
        package_graph: &guppy::graph::PackageGraph,
        package_ids: &[guppy::PackageId],
        duration: std::time::Duration,
    ) {
        if let Some(shell) = pavex_cli_shell::SHELL.get()
            && let Ok(mut shell) = shell.lock()
        {
            for package_id in package_ids {
                let Ok(meta) = package_graph.metadata(package_id) else {
                    continue;
                };
                let _ = shell.status(
                    "Documented",
                    format!(
                        "{}@{} in {:.3} seconds",
                        meta.name(),
                        meta.version(),
                        duration.as_secs_f32()
                    ),
                );
            }
        }
    }
}
