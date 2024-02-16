#[macro_export]
macro_rules! source_or_exit_with_error {
    ($location:ident, $graph:ident, $diagnostics:ident) => {{
        use crate::diagnostic::LocationExt as _;
        match $location.source_file($graph) {
            Ok(s) => s,
            Err(e) => {
                $diagnostics.push(e.into());
                return;
            }
        }
    }};
}
