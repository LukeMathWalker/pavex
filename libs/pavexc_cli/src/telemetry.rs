use std::collections::BTreeMap;

use tracing::{
    field::Visit,
    span::{Attributes, Id, Record},
    Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, EnvFilter, Layer};

/// Keep spans if either of the following is true:
/// - They satisfy the criteria of the inner `EnvFilter`
/// - One of their ancestor spans matched a field filter
pub struct Filtered<L> {
    pub base: EnvFilter,
    pub fields: BTreeMap<String, String>,
    pub layer: L,
}

struct Keep;

impl<L, S> Layer<S> for Filtered<L>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    L: Layer<S>,
{
    fn on_layer(&mut self, subscriber: &mut S) {
        self.layer.on_layer(subscriber);
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>, ctx: Context<'_, S>) -> bool {
        <EnvFilter as tracing_subscriber::layer::Filter<S>>::enabled(&self.base, metadata, &ctx)
            && self.layer.enabled(metadata, ctx)
    }

    fn on_new_span(
        &self,
        attrs: &Attributes<'_>,
        id: &Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).unwrap();
        let mut keep = false;
        if let Some(parent) = span.parent() {
            if parent.extensions().get::<Keep>().is_some() {
                keep = true;
            }
        }

        if !keep {
            // Check if the env filter matches
            if let Some(metadata) = ctx.metadata(id) {
                keep = <EnvFilter as tracing_subscriber::layer::Filter<S>>::enabled(
                    &self.base, metadata, &ctx,
                );
            }
        }

        if !keep {
            let mut visitor = FieldVisitor {
                filters: &self.fields,
                matched: false,
            };
            attrs.values().record(&mut visitor);
            keep = visitor.matched;
        }

        if keep {
            span.extensions_mut().insert(Keep);
            drop(span);
            self.layer.on_new_span(attrs, id, ctx);
        }
    }

    fn on_record(&self, span: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let forward = ctx.span(span).unwrap().extensions().get::<Keep>().is_some();

        if forward {
            self.layer.on_record(span, values, ctx);
        }
    }

    fn on_follows_from(
        &self,
        _span: &Id,
        _follows: &Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // TODO: fix
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let forward = ctx
            .lookup_current()
            .map(|s| s.extensions().get::<Keep>().is_some())
            .unwrap_or(true);
        if forward {
            self.layer.on_event(event, ctx)
        }
    }

    fn event_enabled(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) -> bool {
        ctx.lookup_current()
            .map(|s| s.extensions().get::<Keep>().is_some())
            .unwrap_or(true)
            && self.layer.event_enabled(event, ctx)
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let forward = ctx.span(id).unwrap().extensions().get::<Keep>().is_some();

        if forward {
            self.layer.on_enter(id, ctx);
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let forward = ctx.span(id).unwrap().extensions().get::<Keep>().is_some();

        if forward {
            self.layer.on_exit(id, ctx);
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let forward = ctx.span(&id).unwrap().extensions().get::<Keep>().is_some();

        if forward {
            self.layer.on_close(id, ctx);
        }
    }
}

struct FieldVisitor<'a> {
    filters: &'a BTreeMap<String, String>,
    matched: bool,
}

impl<'a> Visit for FieldVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if self.matched {
            return;
        }

        let Some(expected) = self.filters.get(field.name()) else {
            return;
        };
        // Doubt: is there a less expensive way?
        let value = format!("{value:?}");
        if &value == expected {
            self.matched = true;
        }
    }

    /// Visit a string value.
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if self.matched {
            return;
        }

        let Some(expected) = self.filters.get(field.name()) else {
            return;
        };
        if value == expected {
            self.matched = true;
        }
    }
}
