use std::fmt::Write;

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

/// An alternative to `syn::parse2` that emits an error with span information if the
/// input cannot be parsed.
///
/// `syn::parse2` takes ownership of the input stream, so it is not possible to recover
/// the original input stream in case of an error.
/// The error span information also doesn't map intuitively to the way the original
/// input stream would look like if it was printed.
/// To work around these issues, we convert the input stream to a string before parsing
/// it, so that we can return a meaningful error message in case of a parse error.
/// This is obviously not as efficient as `syn::parse2`, but it is useful for debugging
/// and development purposes. That's why we enable this code path only in debug builds.
pub(crate) fn syn_debug_parse2<T: syn::parse::Parse>(input: proc_macro2::TokenStream) -> T {
    let err_msg = "Failed to parse the generated Rust code as the expected syntax node";
    if cfg!(not(debug_assertions)) {
        syn::parse2(input).expect(err_msg)
    } else {
        let input = input.to_string();
        let err = match syn::parse_str(&input) {
            Ok(parsed) => return parsed,
            Err(e) => e,
        };
        let span = err.span();
        panic!(
            "{err_msg}.\n\
            Input: {input}\n\
            Error: {err}\n\
            Start:\n  Line: {}\n  Column: {}\n\
            End:\n  Line: {}\n  Column: {}",
            span.start().line,
            span.start().column,
            span.end().line,
            span.end().column
        )
    }
}
