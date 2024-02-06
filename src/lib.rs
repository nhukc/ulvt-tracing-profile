//! A span based profiler, utilizing the [tracing](https://docs.rs/tracing/latest/tracing/) crate.
//!
//! # Overview  
//! This implementation of `tracing_subscriber::Layer<S>` records the time
//! a span took to execute, along with any user supplied metadata and
//! information necessary to construct a call graph from the resulting logs.
//!
//! Two Layer implementations are provided:
//!     `CsvLayer`: logs data in CSV format
//!     `PrintTreeLayer`: prints a call graph
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
//!     tracing_subscriber::registry()
//!         .with(PrintTreeLayer::new())
//!         .with(CsvLayer::new("/tmp/output.csv"))
//!         .init();
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
pub use layers::{csv::Layer as CsvLayer, graph::Layer as PrintTreeLayer};

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

    #[test]
    fn all_layers() {
        tracing_subscriber::registry()
            .with(PrintTreeLayer::new())
            .with(CsvLayer::new("/tmp/output.csv"))
            .init();
        make_spans();
    }
}
