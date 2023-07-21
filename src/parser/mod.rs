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
const INVALID_QUERY_INFO: &str = "invalid promql query";
const INDENT_STR: &str = "  ";
const MAX_CHARACTERS_PER_LINE: usize = 100;

/// Approach
/// --------
/// When a PromQL query is parsed, it is converted into PromQL AST,
/// which is a nested structure of nodes. Each node has a depth/level
/// (distance from the root), that is passed by its parent.
///
/// While prettifying, a Node considers 2 things:
/// 1. Did the current Node's parent add a new line?
/// 2. Does the current Node needs to be prettified?
///
/// The level of a Node determines if it should be indented or not.
/// The answer to the 1 is NO if the level passed is 0. This means, the
/// parent Node did not apply a new line, so the current Node must not
/// apply any indentation as prefix.
/// If level > 1, a new line is applied by the parent. So, the current Node
/// should prefix an indentation before writing any of its content. This indentation
/// will be ([level/depth of current Node] * "  ").
///
/// The answer to 2 is YES if the normalized length of the current Node exceeds
/// the [MAX_CHARACTERS_PER_LINE] limit. Hence, it applies the indentation equal to
/// its depth and increments the level by 1 before passing down the child.
/// If the answer is NO, the current Node returns the normalized string value of itself.
pub trait Prettier: std::fmt::Display {
    /// max param is short for max_characters_per_line.
    fn pretty(&self, level: usize, max: usize) -> String {
        if self.needs_split(max) {
            self.format(level, max)
        } else {
            format!("{}{self}", indent(level))
        }
    }

    /// override format if expr needs to be splited into multiple lines
    fn format(&self, level: usize, _max: usize) -> String {
        format!("{}{self}", indent(level))
    }

    /// override needs_split to return false, in order not to split multiple lines
    fn needs_split(&self, max: usize) -> bool {
        self.to_string().len() > max
    }
}

fn indent(n: usize) -> String {
    INDENT_STR.repeat(n)
}
