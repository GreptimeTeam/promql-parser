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
use std::fmt::{self, Display};
use std::time::{Duration, Instant, SystemTime};

use crate::label::Matchers;
use crate::parser::{Function, Token, TokenType};

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
pub struct AggregateExpr {
    pub op: TokenType,         // The used aggregation operation.
    pub expr: Box<Expr>,       // The Vector expression over which is aggregated.
    pub param: Box<Expr>,      // Parameter used by some aggregators.
    pub grouping: Vec<String>, // The labels by which to group the Vector.
    pub without: bool,         // Whether to drop the given labels rather than keep them.
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: TokenType,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
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
pub struct ParenExpr {
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct SubqueryExpr {
    pub expr: Box<Expr>,
    pub offset: Option<Duration>,
    pub start_or_end: Option<TokenType>, // Set when @ is used with start() or end()
    pub range: Duration,
    pub step: Duration,
}

#[derive(Debug, Clone)]
pub struct NumberLiteral {
    pub val: f64,
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub val: String,
}

#[derive(Debug, Clone)]
pub struct VectorSelector {
    pub name: Option<String>,
    pub offset: Option<Duration>,
    pub at: Option<Instant>,
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
    Aggregate(AggregateExpr),

    /// Unary represents a unary operation on another expression.
    /// Currently unary operations are only supported for Scalars.
    Unary(UnaryExpr),

    /// Binary represents a binary expression between two child expressions.
    Binary(BinaryExpr),

    /// Paren wraps an expression so it cannot be disassembled as a consequence
    /// of operator precedence.
    Paren(ParenExpr),

    Subquery(SubqueryExpr),

    NumberLiteral(NumberLiteral),

    StringLiteral(StringLiteral),

    VectorSelector(VectorSelector),

    MatrixSelector(MatrixSelector),

    /// Call represents a function call.
    // TODO: need more descriptions
    Call(Call),
}

impl Expr {
    pub fn new_vector_selector(name: Option<String>, matchers: Matchers) -> Self {
        let vs = VectorSelector {
            name,
            offset: None,
            at: None,
            start_or_end: None,
            label_matchers: matchers,
        };
        Self::VectorSelector(vs)
    }

    pub fn new_unary_expr(expr: Expr, op: &Token) -> Result<Self, String> {
        let ue = match expr {
            Expr::NumberLiteral(number) => Expr::NumberLiteral(NumberLiteral { val: -number.val }),
            _ => Expr::Unary(UnaryExpr {
                op: op.id(),
                expr: Box::new(expr),
            }),
        };
        Ok(ue)
    }

    pub fn new_subquery_expr(expr: Expr, range: Duration, step: Duration) -> Result<Self, String> {
        let se = Expr::Subquery(SubqueryExpr {
            expr: Box::new(expr),
            offset: None,
            start_or_end: None,
            range,
            step,
        });
        Ok(se)
    }

    pub fn new_matrix_selector(expr: Expr, range: Duration) -> Result<Self, String> {
        match expr {
            Expr::VectorSelector {
                offset: Some(_), ..
            } => Err("".into()),
            Expr::VectorSelector { at: Some(_), .. } => Err("".into()),
            Expr::VectorSelector { .. } => {
                let ms = Expr::MatrixSelector {
                    vector_selector: Box::new(expr),
                    range,
                };
                Ok(ms)
            }
            _ => Err("".into()),
        }
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
