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

//! The parser implementation.
//!
//! [`parse()`] parses the given query to [`Expr`], which is the abstract syntax tree (AST) struct
//! in this crate. And [`Expr`] is componsed by servaral structs exposed in this module.
//!
//! Notes that in PromQL the parsed [`Expr`] is only a part of an query. It would also needs other
//! parameters like "start"/"end" time or "step" time etc, which is included in [`EvalStmt`].

pub mod ast;
pub mod function;
pub mod lex;
pub mod parse;
pub mod production;
pub mod token;
pub mod value;

pub use ast::{
    AggregateExpr, AtModifier, BinModifier, BinaryExpr, Call, EvalStmt, Expr, Extension,
    LabelModifier, MatrixSelector, NumberLiteral, Offset, ParenExpr, StringLiteral, SubqueryExpr,
    UnaryExpr, VectorMatchCardinality, VectorSelector,
};

pub use function::{Function, FunctionArgs};
pub use lex::{lexer, LexemeType};
pub use parse::parse;
pub use token::{Token, TokenId, TokenType};
pub use value::{Value, ValueType};

// FIXME: show more helpful error message to some invalid promql queries.
pub const INVALID_QUERY_INFO: &str = "invalid promql query";
