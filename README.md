# PromQL Lexer and Parser for Rust

The goal of this project is to build a PromQL lexer and parser capable of
parsing PromQL that conforms with [Prometheus Query][querying-prometheus].

## Example

TODO

## PromQL compliance

This crate declares compatible with [prometheus 0372e25][prom-0372e25], which is
prometheus release 2.40 at Nov 29, 2022. Any revision on PromQL after this
commit is not guaranteed.

## Design

TODO

## Contributing

Contributions are highly encouraged!

Pull requests that add support for or fix a bug in a feature in the PromQL will
likely be accepted after review.

## Licensing

All code in this repository is licensed under the [Apache License 2.0](LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.

[prom-0372e25]: https://github.com/prometheus/prometheus/tree/0372e259baf014bbade3134fd79bcdfd8cbdef2c
[querying-prometheus]: https://prometheus.io/docs/prometheus/latest/querying/basics/
