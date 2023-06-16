use pavex::blueprint::reflection::Location;

pub trait ProcMacroSpanExt {
    /// Returns `true` if a `proc_macro::Span` contains a `Location`.
    /// `false` otherwise.
    ///
    /// Important: it doesn't take into account the `Location`'s file path.
    /// You must check beforehand that the `Location`'s file path is the same as the
    /// `proc_macro::Span`'s file path.
    fn contains(&self, location: &Location) -> bool;
}

impl ProcMacroSpanExt for proc_macro2::Span {
    fn contains(&self, location: &Location) -> bool {
        let span_start = self.start();
        if span_start.line < location.line as usize
            || (span_start.line == location.line as usize
                && span_start.column <= location.column as usize)
        {
            let span_end = self.end();
            if span_end.line > location.line as usize
                || (span_end.line == location.line as usize
                    && span_end.column >= location.column as usize)
            {
                return true;
            }
        }
        false
    }
}
