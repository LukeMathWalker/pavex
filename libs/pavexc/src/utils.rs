use std::fmt::Write;
use std::fmt::{Display, Formatter};

/// Render a list of items separated by commas, with an "and" before the last item.
///
/// E.g. `a, b, c and d`.
pub(crate) fn comma_separated_list<'a, T, I>(
    mut buffer: impl Write,
    mut iter: I,
    f: impl Fn(&'a T) -> String,
    conjunction: &str,
) -> Result<(), std::fmt::Error>
where
    T: 'a,
    I: Iterator<Item = &'a T> + ExactSizeIterator,
{
    let length = iter.len();
    if length == 1 {
        write!(buffer, "{}", &f(iter.next().unwrap()))?;
        return Ok(());
    }
    for (i, item) in iter.enumerate() {
        if i == length.saturating_sub(1) {
            write!(buffer, " {conjunction} ")?;
        }
        write!(buffer, "{}", &f(item))?;
        if i < length.saturating_sub(2) {
            write!(buffer, ", ")?;
        }
    }
    Ok(())
}

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
