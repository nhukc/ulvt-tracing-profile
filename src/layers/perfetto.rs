use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::mpsc;
use std::{collections::BTreeMap, time::Instant};
use tracing::span;

use crate::data::{self, with_span_storage_mut, FieldVisitor};
use crate::err_msg;

pub struct Layer {
    _perfetto_guard: Option<perfetto_sys::PerfettoGuard>,
}

impl Layer {
    pub fn new<T: AsRef<Path>>() -> Self {
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
        id: &span::Id,
        values: &span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
    }

    fn on_enter(&self, id: &span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let Some(span) = ctx.span(id) else {
            err_msg!("failed to get span on_enter");
            return;
        };
        with_span_storage_mut::<CsvMetadata, _>(id, ctx, |storage| {
            storage
                .trace_guard
                .replace(perfetto_sys::TraceEvent::new(span.name()))
        });
    }

    fn on_exit(&self, id: &span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let parent = span.parent();
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
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let Some(span) = ctx.span(id) else {
            err_msg!("failed to get span on_new_span");
            return;
        };

        let mut storage = PerfettoMetadata { trace_guard: None };
        let mut extensions = span.extensions_mut();
        extensions.insert(storage);
    }
}
