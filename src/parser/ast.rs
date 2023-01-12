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

#![allow(dead_code)]
use lrpar::Span;
use std::fmt::{self, Display};
use std::time::{Duration, SystemTime};

use crate::label::Matchers;
use crate::parser::{Function, TokenType};

/// EvalStmt holds an expression and information on the range it should
/// be evaluated on.
#[derive(Debug, Clone)]
pub struct EvalStmt {
    pub expr: Expr, // Expression to be evaluated.

    // The time boundaries for the evaluation. If start equals end an instant
    // is evaluated.
    pub start: SystemTime,
    pub end: SystemTime,
    // Time between two evaluated instants for the range [start:end].
    pub interval: Duration,
    // Lookback delta to use for this evaluation.
    pub lookback_delta: Duration,
}

#[derive(Debug, Clone)]
pub struct Aggregate {
    pub op: TokenType,         // The used aggregation operation.
    pub expr: Box<Expr>,       // The Vector expression over which is aggregated.
    pub param: Box<Expr>,      // Parameter used by some aggregators.
    pub grouping: Vec<String>, // The labels by which to group the Vector.
    pub without: bool,         // Whether to drop the given labels rather than keep them.
}

#[derive(Debug, Clone)]
pub struct Unary {
    pub op: TokenType,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct Binary {
    pub op: TokenType,  // The operation of the expression.
    pub lhs: Box<Expr>, // The operands on the left sides of the operator.
    pub rhs: Box<Expr>, // The operands on the right sides of the operator.

    // The matching behavior for the operation if both operands are Vectors.
    // If they are not this field is None.
    pub matching: Option<VectorMatching>,

    // If a comparison operator, return 0/1 rather than filtering.
    pub return_bool: bool,
}

#[derive(Debug, Clone)]
pub struct Paren {
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct Subquery {
    pub expr: Box<Expr>,
    pub range: Duration,
    pub offset: Duration,
    pub timestamp: Option<i64>,
    pub start_or_end: TokenType, // Set when @ is used with start() or end()
    pub step: Duration,
}

#[derive(Debug, Clone)]
pub struct NumberLiteral {
    pub val: f64,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub val: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VectorSelector {
    pub name: Option<String>,
    // offset is the actual offset that was set in the query.
    // This never changes.
    pub offset: Option<Duration>,
    pub start_or_end: Option<TokenType>, // Set when @ is used with start() or end()
    pub label_matchers: Matchers,
}

#[derive(Debug, Clone)]
pub struct MatrixSelector {
    // It is safe to assume that this is an VectorSelector
    // if the parser hasn't returned an error.
    pub vector_selector: Box<Expr>,
    pub range: Duration,
}

#[derive(Debug, Clone)]
pub struct Call {
    pub func: Function,       // The function that was called.
    pub args: Vec<Box<Expr>>, // Arguments used in the call.
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// Aggregate represents an aggregation operation on a Vector.
    Aggregate(Aggregate),

    /// Unary represents a unary operation on another expression.
    /// Currently unary operations are only supported for Scalars.
    Unary(Unary),

    /// Binary represents a binary expression between two child expressions.
    Binary(Binary),

    /// Paren wraps an expression so it cannot be disassembled as a consequence
    /// of operator precedence.
    Paren(Paren),

    Subquery(Subquery),

    NumberLiteral(NumberLiteral),

    StringLiteral(StringLiteral),

    VectorSelector(VectorSelector),

    MatrixSelector(MatrixSelector),

    /// Call represents a function call.
    // TODO: need more descriptions
    Call(Call),
}

impl Expr {
    pub fn empty_vector_selector() -> Self {
        let vs = VectorSelector {
            name: None,
            offset: None,
            start_or_end: None,
            label_matchers: Matchers::empty(),
        };
        Self::VectorSelector(vs)
    }

    pub fn new_vector_selector(name: Option<String>, matchers: Matchers) -> Self {
        let vs = VectorSelector {
            name,
            offset: None,
            start_or_end: None,
            label_matchers: matchers,
        };
        Self::VectorSelector(vs)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VectorMatchCardinality {
    OneToOne,
    ManyToOne,
    OneToMany,
    ManyToMany,
}

impl Display for VectorMatchCardinality {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            VectorMatchCardinality::OneToOne => write!(f, "one-to-one"),
            VectorMatchCardinality::ManyToOne => write!(f, "many-to-one"),
            VectorMatchCardinality::OneToMany => write!(f, "one-to-many"),
            VectorMatchCardinality::ManyToMany => write!(f, "many-to-many"),
        }
    }
}

// VectorMatching describes how elements from two Vectors in a binary
// operation are supposed to be matched.
#[derive(Debug, Clone)]
pub struct VectorMatching {
    // The cardinality of the two Vectors.
    pub card: VectorMatchCardinality,
    // MatchingLabels contains the labels which define equality of a pair of
    // elements from the Vectors.
    pub matching_labels: Vec<String>,
    // On includes the given label names from matching,
    // rather than excluding them.
    pub on: bool,
    // Include contains additional labels that should be included in
    // the result from the side with the lower cardinality.
    pub include: Vec<String>,
}
