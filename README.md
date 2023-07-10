[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/GreptimeTeam/promql-parser/blob/main/LICENSE)
[![Build Status](https://github.com/greptimeteam/promql-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/GreptimeTeam/promql-parser/blob/main/.github/workflows/ci.yml)
[![Version](https://img.shields.io/crates/v/promql-parser?label=promql-parser)](https://crates.io/crates/promql-parser)
[![codecov](https://codecov.io/gh/GreptimeTeam/promql-parser/branch/main/graph/badge.svg?token=4GEPVMJYNG)](https://app.codecov.io/gh/GreptimeTeam/promql-parser/tree/main)


# PromQL Lexer and Parser

The goal of this project is to build a PromQL lexer and parser capable of
parsing PromQL that conforms with [Prometheus Query][querying-prometheus].

## Example

To parse a simple instant vector selector expression:

``` rust
use promql_parser::parser;

let promql = r#"http_requests_total{environment=~"staging|testing|development",method!="GET"} @ 1609746000 offset 5m"#;

match parser::parse(promql) {
    Ok(ast) => println!("AST: {:?}", ast),
    Err(info) => println!("Err: {:?}", info),
}
```

or you can directly run examples under this repo:

``` shell
cargo run --example parser
```

This outputs:

```rust
AST: VectorSelector(VectorSelector { name: Some("http_requests_total"), matchers: Matchers { matchers: {Matcher { op: NotEqual, name: "method", value: "GET" }, Matcher { op: Re(staging|testing|development), name: "environment", value: "staging|testing|development" }, Matcher { op: Equal, name: "__name__", value: "http_requests_total" }} }, offset: Some(Pos(300s)), at: Some(At(SystemTime { tv_sec: 1609746000, tv_nsec: 0 })) })
```

## PromQL compliance

This crate declares compatible with [prometheus v2.45.0][prom-v2.45.0], which is
released at 2023-06-23. Any revision on PromQL after this commit is not guaranteed.

## Community Extensions

There are a number of community projects that extend promql-parser or
provide integrations with other systems.

### Language Bindings

- [py-promql-parser](https://github.com/messense/py-promql-parser) Python binding of this crate.

## Known Uses

Here are some of the projects known to use promql-parser:

- [GreptimeDB](https://github.com/GreptimeTeam/greptimedb) Open Source & Cloud Native Distributed Time Series Database

If your project is using promql-parser, feel free to make a PR to add it to this list.

## Contributing

Contributions are highly encouraged!

Pull requests that add support for or fix a bug in a feature in the PromQL will
likely be accepted after review.

## Licensing

All code in this repository is licensed under the [Apache License 2.0](LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.

[prom-v2.45.0]: https://github.com/prometheus/prometheus/tree/v2.45.0
[querying-prometheus]: https://prometheus.io/docs/prometheus/latest/querying/basics/
