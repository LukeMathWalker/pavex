use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{
        self,
        format::{Format, Writer},
        time::FormatTime,
        FmtContext, FormatEvent, FormatFields,
    },
    registry::LookupSpan,
};

pub struct ReversedFull;

impl<S, N, T> FormatEvent<S, N> for Format<ReversedFull, T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let normalized_meta = event.normalized_metadata();
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());

        // if the `Format` struct *also* has an ANSI color configuration,
        // override the writer...the API for configuring ANSI color codes on the
        // `Format` struct is deprecated, but we still need to honor those
        // configurations.
        writer = writer.with_ansi(ansi);

        self.format_timestamp(&mut writer)?;

        let fmt_level = { FmtLevel::new(meta.level(), writer.has_ansi_escapes()) };
        write!(writer, "{} ", fmt_level)?;
        let dimmed = writer.dimmed();

        if let Some(scope) = ctx.event_scope() {
            let bold = writer.bold();

            let mut seen = false;

            for span in scope.from_root() {
                write!(writer, "{}", bold.paint(span.metadata().name()))?;
                seen = true;

                let ext = span.extensions();
                if let Some(fields) = &ext.get::<FormattedFields<N>>() {
                    if !fields.is_empty() {
                        write!(writer, "{}{}{}", bold.paint("{"), fields, bold.paint("}"))?;
                    }
                }
                write!(writer, "{}", dimmed.paint(":"))?;
            }

            if seen {
                writer.write_char(' ')?;
            }
        };

        write!(
            writer,
            "{}{} ",
            dimmed.paint(meta.target()),
            dimmed.paint(":")
        )?;

        let line_number = meta.line();
        if let Some(filename) = meta.file() {
            write!(
                writer,
                "{}{}{}",
                dimmed.paint(filename),
                dimmed.paint(":"),
                if line_number.is_some() { "" } else { " " }
            )?;
        }

        write!(
            writer,
            "{}{}:{} ",
            dimmed.prefix(),
            line_number,
            dimmed.suffix()
        )?;

        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}
