#[macro_export]
macro_rules! try_source {
    ($location:ident, $graph:ident, $diagnostics:ident) => {{
        use crate::diagnostic::LocationExt as _;
        match $location.source_file($graph) {
            Ok(s) => Some(s),
            Err(e) => {
                $diagnostics.push(e.into());
                None
            }
        }
    }};
}
