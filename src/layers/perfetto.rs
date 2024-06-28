use tracing::span;

use crate::data::{with_span_storage_mut, PerfettoMetadata};
use crate::err_msg;

pub struct Layer {
    _perfetto_guard: Option<perfetto_sys::PerfettoGuard>,
}

impl Default for Layer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer {
    pub fn new() -> Self {
        Self {
            _perfetto_guard: Some(perfetto_sys::PerfettoGuard::new()),
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for Layer
where
    S: tracing::Subscriber,
    // no idea what this is but it lets you access the parent span.
    S: for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    // handles log events like debug!
    fn on_event(
        &self,
        _event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // don't care about these
    }

    fn on_record(
        &self,
        _id: &span::Id,
        _values: &span::Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
    }

    fn on_enter(&self, id: &span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span_name = match ctx.span(id) {
            Some(span) => span.name(),
            None => {
                err_msg!("failed to get span on_enter");
                return;
            }
        };
        with_span_storage_mut::<PerfettoMetadata, _>(id, ctx, |storage| {
            storage
                .trace_guard
                .replace(perfetto_sys::TraceEvent::new(span_name));
        });
    }

    fn on_exit(&self, id: &span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            if let Some(storage) = span.extensions_mut().get_mut::<PerfettoMetadata>() {
                storage.trace_guard.take();
            } else {
                err_msg!("failed to get storage on_exit");
            }
        } else {
            err_msg!("failed to get span on_exit");
        }
    }

    fn on_new_span(
        &self,
        _attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let Some(span) = ctx.span(id) else {
            err_msg!("failed to get span on_new_span");
            return;
        };

        let storage = PerfettoMetadata { trace_guard: None };
        let mut extensions = span.extensions_mut();
        extensions.insert(storage);
    }
}
