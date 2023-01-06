use pavex_builder::Location;

pub trait ProcMacroSpanExt {
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
