use owo_colors::{OwoColorize, Style};
use tracing::{Event, Level, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::fmt::FormattedFields;
use tracing_subscriber::{
    fmt::{format::Writer, FmtContext, FormatEvent, FormatFields},
    registry::LookupSpan,
};

pub struct ReversedFull;

impl<S, N> FormatEvent<S, N> for ReversedFull
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        use std::fmt::Write as _;

        let normalized_meta = event.normalized_metadata();
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());

        let mut indentantion_level = 0;
        if let Some(scope) = ctx.event_scope() {
            for span in scope {
                if span.extensions().get::<FormattedFields<N>>().is_some() {
                    indentantion_level += 1;
                }
            }
        }

        let mut buffer = String::new();

        match *meta.level() {
            Level::ERROR => {
                write!(&mut buffer, "{}", "ERROR ".red().bold())?;
            }
            Level::WARN => {
                write!(&mut buffer, "{}", "WARN  ".yellow().bold())?;
            }
            Level::INFO => {
                write!(&mut buffer, "{}", "INFO  ".green().bold())?;
            }
            Level::DEBUG => {
                write!(&mut buffer, "{}", "DEBUG ".cyan().bold())?;
            }
            Level::TRACE => {
                write!(&mut buffer, "{}", "TRACE ".dimmed())?;
            }
        }

        let dimmed = Style::new().dimmed();
        let bold = Style::new().bold();

        if let Some(mut scope) = ctx.event_scope() {
            if let Some(span) = scope.next() {
                write!(&mut buffer, "{}\n ", bold.style(span.metadata().name()))?;

                let mut sub_writer = tracing_subscriber::fmt::format::Writer::new(&mut buffer);
                ctx.format_fields(sub_writer.by_ref(), event)?;
                writeln!(&mut buffer)?;

                let ext = span.extensions();
                if let Some(fields) = &ext.get::<FormattedFields<N>>() {
                    if !fields.is_empty() {
                        writeln!(&mut buffer, " {} {}", dimmed.style("with"), fields)?;
                    }
                }
            }
        }

        writeln!(&mut buffer, " target: {}", dimmed.style(meta.target()))?;

        let line_number = meta.line();
        if let Some(filename) = meta.file() {
            write!(&mut buffer, " {}", dimmed.style(filename),)?;
            if let Some(line_number) = line_number {
                write!(
                    &mut buffer,
                    "{} {}",
                    dimmed.style(":"),
                    dimmed.style(line_number),
                )?;
            }
            writeln!(&mut buffer)?;
        }

        writeln!(
            writer,
            "{}",
            textwrap::indent(&buffer, &"  ".repeat(indentantion_level))
        )
    }
}
