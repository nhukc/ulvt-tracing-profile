use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
    sync::Mutex,
    time::Instant,
};

use once_cell::sync::Lazy;
use tracing::span;

/// GraphLayer (internally called layer::graph)  
/// This Layer prints a call graph to stdout
///
/// example output:
/// ```bash
/// cargo test all_layers -- --nocapture
///
/// running 1 test
/// | root span; 0.145110 ms; 100.000 %, {}
/// | | child span1; 0.005400 ms; 3.721 %, {"field1":"value1"}
/// | | child span2; 0.077458 ms; 53.379 %, {"field2":"value2"}
/// | | | child span3; 0.003180 ms; 2.191 %, {"field3":"value3"}
/// | | | child span4; 0.003175 ms; 2.188 %, {"field4":"value4"}
/// test tests::graph_layer1 ... ok
/// ```
#[derive(Default)]
pub struct Layer;

impl Layer {
    pub fn new() -> Self {
        Self {}
    }
}

use crate::data::{self, FieldVisitor};
use crate::err_msg;

static GRAPH: Lazy<Mutex<Graph>> = Lazy::new(|| Mutex::new(Graph::default()));

struct SpanMetadata(data::SpanMetadata);

impl Deref for SpanMetadata {
    type Target = data::SpanMetadata;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SpanMetadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default)]
struct Graph {
    children: HashMap<u64, Vec<GraphNode>>,
}

impl Graph {
    fn print_tree(&self, node: &GraphNode, root_time_ns: u64) {
        node.print_self(root_time_ns);

        if let Some(list) = self.children.get(&node.id) {
            for child in list {
                self.print_tree(child, root_time_ns)
            }
        };
    }
}

#[derive(Debug)]
struct GraphNode {
    name: String,
    id: u64,
    execution_time_ns: u64,
    metadata: BTreeMap<String, String>,
    call_depth: u64,
}

impl GraphNode {
    fn print_self(&self, root_time_ns: u64) {
        let pipes: Vec<&str> = (0..self.call_depth).map(|_| "|").collect();
        let pipe_str = pipes.join(" ");
        let relative_time = self.execution_time_ns as f64 * 100.0 / root_time_ns as f64;

        let metadata = if !self.metadata.is_empty() {
            let kv: Vec<_> = self
                .metadata
                .iter()
                .map(|(k, v)| format!("\"{k}\":\"{v}\""))
                .collect();
            format!("; {{{}}}", kv.join(", "))
        } else {
            String::new()
        };
        let execution_time = self.execution_time_ns as f64 / 1000000.0;

        println!(
            "{pipe_str} {}; {:.6} ms; {:.3} %{}",
            self.name, execution_time, relative_time, metadata
        )
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
        if let Some(span) = ctx.span(id) {
            if let Some(storage) = span.extensions_mut().get_mut::<SpanMetadata>() {
                let mut visitor = FieldVisitor(&mut storage.fields);
                values.record(&mut visitor);
            } else {
                err_msg!("failed to get storage on_record");
            }
        } else {
            err_msg!("failed to get span on_record");
        }
    }

    fn on_enter(&self, id: &span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            if let Some(storage) = span.extensions_mut().get_mut::<SpanMetadata>() {
                storage.start_time.replace(Instant::now());
            } else {
                err_msg!("failed to get storage on_enter");
            }
        } else {
            err_msg!("failed to get span on_enter");
        }
    }

    fn on_exit(&self, id: &span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let parent = span.parent();
            let (elapsed_ns, call_depth, fields) =
                match span.extensions_mut().get_mut::<SpanMetadata>() {
                    Some(storage) => {
                        let elapsed_ns = storage
                            .start_time
                            .map(|x| x.elapsed().as_nanos() as u64)
                            .unwrap_or_default();

                        let fields = std::mem::take(&mut storage.fields);
                        (elapsed_ns, storage.call_depth, fields)
                    }
                    None => {
                        err_msg!("failed to get storage on_exit");
                        return;
                    }
                };

            let graph_node = GraphNode {
                id: span.id().into_u64(),
                execution_time_ns: elapsed_ns,
                name: span.name().into(),
                call_depth,
                metadata: fields,
            };

            match parent {
                Some(p) => {
                    let parent_id = p.id().into_u64();
                    let mut graph = match GRAPH.lock() {
                        Ok(r) => r,
                        Err(e) => {
                            err_msg!("failed to get mutex: {e}");
                            return;
                        }
                    };

                    let values = graph.children.entry(parent_id).or_insert_with(Vec::new);
                    values.push(graph_node);
                }
                None => {
                    let graph = match GRAPH.lock() {
                        Ok(r) => r,
                        Err(e) => {
                            err_msg!("failed to get mutex: {e}");
                            return;
                        }
                    };

                    graph.print_tree(&graph_node, elapsed_ns);
                }
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

        let parent_call_depth = span
            .parent()
            .as_ref()
            .and_then(|p| p.extensions().get::<SpanMetadata>().map(|x| x.call_depth))
            .unwrap_or_default();

        let mut storage = SpanMetadata(data::SpanMetadata {
            start_time: None,
            call_depth: parent_call_depth + 1,
            fields: BTreeMap::new(),
        });

        // warning: the library user must use #[instrument(skip_all)] or else too much data will be logged
        let mut visitor = FieldVisitor(&mut storage.fields);
        attrs.record(&mut visitor);

        let mut extensions = span.extensions_mut();
        extensions.insert(storage);
    }
}
