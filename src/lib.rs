//! A span based profiler, utilizing the [tracing](https://docs.rs/tracing/latest/tracing/) crate.
//!
//! # Overview
//! This implementation of `tracing_subscriber::Layer<S>` records the time
//! a span took to execute, along with any user supplied metadata and
//! information necessary to construct a call graph from the resulting logs.
//!
//! Four `Layer` implementations are provided:
//!     `CsvLayer`: logs data in CSV format
//!     `PrintTreeLayer`: prints a call graph
//!     `PrintPerfCountersLayer`: prints aggregated performance counters for each span.
//!     `PerfettoLayer`: Connects to a system-wide perfetto logging service which will create a fused trace. Be warned - the program will block until a connection is established with perfetto's traced service.
//!
//! ```
//! use tracing::instrument;
//! use tracing::debug_span;
//! use tracing_subscriber::prelude::*;
//! use tracing_profile::*;
//!
//! #[instrument(skip_all, name= "graph_root", fields(a="b", c="d"))]
//! fn entry_point() {
//!     let span = debug_span!("some_span");
//!     let _scope1 = span.enter();
//!
//!     let span2 = debug_span!("another_span", field1 = "value1");
//!     let _scope2 = span2.enter();
//! }
//!
//! fn main() {
//!     let layer = tracing_subscriber::registry()
//!         .with(PrintTreeLayer::default())
//!         .with(CsvLayer::new("/tmp/output.csv"));
//!
//!     // note that both these features could be used at once. The code is written this way to make the rustdoc compile.
//!     loop {
//!         #[cfg(feature = "perfetto")]
//!         {
//!             layer.with(PerfettoLayer::new()).init();
//!             
//!             // all spans will be included in the fused trace. additionally the user may use this zkprof specific counter as follows:
//!             // note that units are in Gb/s
//!             record_fpga_throughput("card1", 100.5);
//!             break;
//!         }
//!         
//!         #[cfg(feature = "perf_counters")]
//!         {
//!             use perf_event::events::Hardware;
//!             layer.with(PrintPerfCountersLayer::new(
//!                 vec![("instructions".to_string(), Hardware::INSTRUCTIONS.into())]
//!             ).unwrap()).init();
//!             break;
//!         }
//!         
//!         layer.init();
//!         break;
//!     }
//!     
//!     entry_point();
//! }
//! ```
//!
//! Note that if `#[instrument]` is used, `skip_all` is recommended. Omitting this will result in
//! all the function arguments being included as fields.
//!
//! # Features
//! The `panic` feature will turn eprintln! into panic!, causing the program to halt on errors.

mod data;
mod layers;

#[cfg(feature = "perf_counters")]
pub use layers::print_perf_counters::Layer as PrintPerfCountersLayer;
pub use layers::{
    csv::Layer as CsvLayer,
    graph::{Config as PrintTreeConfig, Layer as PrintTreeLayer},
};

#[cfg(feature = "perfetto")]
pub use layers::perfetto::Layer as PerfettoLayer;

#[cfg(feature = "perfetto")]
pub use perfetto_sys::record_fpga_throughput;

// use this instead of eprintln!
macro_rules! err_msg {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        assert!(cfg!(not(feature = "panic")))
    }};
}

pub(crate) use err_msg;

#[cfg(test)]
mod tests {
    use tracing::debug_span;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::prelude::*;

    use super::*;

    fn make_spans() {
        let span = debug_span!("root span");
        let _scope1 = span.enter();

        // child spans 1 and 2 are siblings
        let span2 = debug_span!("child span1", field1 = "value1");
        let scope2 = span2.enter();
        drop(scope2);

        let span3 = debug_span!("child span2", field2 = "value2");
        let _scope3 = span3.enter();

        // child spans 3 and 4 are siblings
        let span = debug_span!("child span3", field3 = "value3");
        let scope = span.enter();
        drop(scope);

        let span = debug_span!("child span4", field4 = "value4");
        let scope = span.enter();
        drop(scope);
    }

    #[cfg(not(feature = "perf_counters"))]
    fn with_with_perf_counters(subscriber: impl SubscriberExt) -> impl SubscriberExt {
        subscriber
    }

    #[cfg(feature = "perf_counters")]
    fn with_with_perf_counters<S>(subscriber: S) -> impl SubscriberExt
    where
        S: SubscriberExt + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
    {
        use perf_event::events::Hardware;

        subscriber.with(
            PrintPerfCountersLayer::new(vec![
                ("instructions".to_string(), Hardware::INSTRUCTIONS.into()),
                ("cycles".to_string(), Hardware::CPU_CYCLES.into()),
            ])
            .unwrap(),
        )
    }

    #[test]
    fn all_layers() {
        with_with_perf_counters(
            tracing_subscriber::registry()
                .with(PrintTreeLayer::default())
                .with(CsvLayer::new("/tmp/output.csv")),
        )
        .init();
        make_spans();
    }

    #[cfg(feature = "perfetto")]
    #[test]
    fn perfetto_test() {
        tracing_subscriber::registry()
            .with(PerfettoLayer::new())
            .init();
        make_spans();

        record_fpga_throughput("fpga1", 1.8);
    }
}
