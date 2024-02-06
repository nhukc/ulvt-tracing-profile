use std::{collections::BTreeMap, time::Instant};

#[derive(Debug)]
pub struct SpanMetadata {
    pub start_time: Option<Instant>,
    pub call_depth: u64,
    pub fields: BTreeMap<String, String>,
}
