use std::fmt::{Display, Formatter};

pub fn anyhow2miette(err: anyhow::Error) -> miette::Error {
    #[derive(Debug, miette::Diagnostic)]
    struct InteropError(anyhow::Error);

    impl Display for InteropError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl std::error::Error for InteropError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.0.source()
        }
    }

    miette::Error::from(InteropError(err))
}
