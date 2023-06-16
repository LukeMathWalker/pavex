use std::fmt::Write;

/// Render a list of items separated by commas, with an "and" before the last item.
///
/// E.g. `a, b, c and d`.
pub(crate) fn comma_separated_list<'a, T, I>(
    mut buffer: impl Write,
    iter: I,
    f: impl Fn(&'a T) -> String,
    conjunction: &str,
) -> Result<(), std::fmt::Error>
where
    T: 'a,
    I: Iterator<Item = &'a T> + ExactSizeIterator,
{
    let length = iter.len();
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
