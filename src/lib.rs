// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # PromQL Lexer and Parser
//!
//! The goal of this project is to build a PromQL lexer and parser capable of
//! parsing PromQL that conforms with [Prometheus Query][querying-prometheus].
//!
//! ## Example
//!
//! The parser entry point is [`parser::parse()`], which takes a string slice of Promql
//! and returns the parse result, either an AST ([`parser::Expr`]) or an error message.
//! Other query parameters like time range and step are included in [`parser::EvalStmt`].
//!
//! ``` rust
//! use promql_parser::parser;
//!
//! let promql = r#"http_requests_total{environment=~"staging|testing|development",method!="GET"} offset 5m"#;
//!
//! match parser::parse(promql, None) {
//!     Ok(expr) => {
//!         println!("Prettify:\n\n{}", expr.prettify());
//!         println!("AST:\n{expr:?}");
//!     }
//!     Err(info) => println!("Err: {info:?}"),
//! }
//! ```
//!
//! or you can directly run examples under this repo:
//!
//! ``` shell
//! cargo run --example parser
//! ```
//!
//! This outputs:
//!
//! ```rust, ignore
//! Prettify:
//! http_requests_total{environment=~"staging|testing|development",method!="GET"} offset 5m
//!
//! AST:
//! VectorSelector(VectorSelector { name: Some("http_requests_total"), matchers: Matchers { matchers: [Matcher { op: Re(staging|testing|development), name: "environment", value: "staging|testing|development" }, Matcher { op: NotEqual, name: "method", value: "GET" }] }, offset: Some(Pos(300s)), at: None })
//! ```
//! ## PromQL compliance
//!
//! This crate declares compatible with [prometheus v2.45.0][prom-v2.45.0], which is
//! released at 2023-06-23. Any revision on PromQL after this commit is not guaranteed.
//!
//! [prom-v2.45.0]: https://github.com/prometheus/prometheus/tree/v2.45.0
//! [querying-prometheus]: https://prometheus.io/docs/prometheus/latest/querying/basics/

#![allow(clippy::let_unit_value)]
lrpar::lrpar_mod!("parser/promql.y");

pub mod label;
pub mod parser;
pub mod util;
