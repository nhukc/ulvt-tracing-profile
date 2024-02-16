# tracing-profile

This library implements subscribers for the [`tracing`](https://docs.rs/tracing/latest/tracing/) crate that facilitate
profiling of one-shot program executions. That is, this is not intended for profiling of long-running programs.

`tracing-profile` depends on [`tracing-subscriber`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/) and
implements layers that measure, record, and display timing information in the span graph. The implemented
`tracing_subscriber::Layer` records the time a span took to execute, along with any user supplied metadata and
information necessary to construct a span graph from the resulting logs.

Note that if `#[tracing::instrument]` is used, the `skip_all` argument is recommended. Omitting this will result in all
the function arguments being included as fields.

## Usage

The library exposes two layers that output the information in different ways.

### CsvLayer

The `CsvLayer` writes profiling information to a CSV file, which can be analyzed later by reconstructing the span graph.

```
$ cargo test
$ cat /tmp/output.csv
id,parent_id,elapsed_ns,span_name,file_name,call_depth,metadata
2,1,22837,child span1,src/lib.rs,2,{"field1":"value1"}
4,3,9255,child span3,src/lib.rs,3,{"field3":"value3"}
5,3,7135,child span4,src/lib.rs,3,{"field4":"value4"}
3,1,119802,child span2,src/lib.rs,2,{"field2":"value2"}
1,0,287881,root span,src/lib.rs,1,{}
```

### PrintTreeLayer

The `PrintTreeLayer` processes the profiling information in the running process and prints the timing information in a
human-readable format. The output structures the span graph as a tree.

```
$ cargo test -- --nocapture
| root span; 0.234570 ms; 100.000 %
| | child span1; 0.006450 ms; 2.750 %; {"field1":"value1"}
| | child span2; 0.112323 ms; 47.885 %; {"field2":"value2"}
| | | child span3; 0.005181 ms; 2.209 %; {"field3":"value3"}
| | | child span4; 0.004678 ms; 1.994 %; {"field4":"value4"}
```

### Example Test

```rust
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
```

## Authors

`tracing-profile` is developed and maintained by [Ulvetanna](https://www.ulvetanna.io).

## License

MIT License

Copyright (c) 2024 Ulvetanna, Inc

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.