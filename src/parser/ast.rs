// Copyright 2022 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::{self, Display};
use std::time::{Duration, Instant};

use crate::label::Matcher;
use crate::parser::Function;
use crate::parser::ItemType;

// EvalStmt holds an expression and information on the range it should
// be evaluated on.
pub struct EvalStmt {
    expr: Expr, // Expression to be evaluated.

    // The time boundaries for the evaluation. If start equals end an instant
    // is evaluated.
    start: Instant,
    end: Instant,
    // Time between two evaluated instants for the range [start:end].
    interval: Duration,
    // Lookback delta to use for this evaluation.
    lookback_delta: Duration,
}

#[derive(Debug)]
pub enum Expr {
    // AggregateExpr represents an aggregation operation on a Vector.
    AggregateExpr {
        op: ItemType,          // The used aggregation operation.
        expr: Box<Expr>,       // The Vector expression over which is aggregated.
        param: Box<Expr>,      // Parameter used by some aggregators.
        grouping: Vec<String>, // The labels by which to group the Vector.
        without: bool,         // Whether to drop the given labels rather than keep them.
    },

    // UnaryExpr represents a unary operation on another expression.
    // Currently unary operations are only supported for Scalars.
    UnaryExpr {
        op: ItemType,
        expr: Box<Expr>,
    },

    // BinaryExpr represents a binary expression between two child expressions.
    BinaryExpr {
        op: ItemType,   // The operation of the expression.
        lhs: Box<Expr>, // The operands on the left sides of the operator.
        rhs: Box<Expr>, // The operands on the right sides of the operator.

        // The matching behavior for the operation if both operands are Vectors.
        // If they are not this field is None.
        matching: Option<VectorMatching>,

        // If a comparison operator, return 0/1 rather than filtering.
        return_bool: bool,
    },

    // ParenExpr wraps an expression so it cannot be disassembled as a consequence
    // of operator precedence.
    ParenExpr {
        expr: Box<Expr>,
    },

    // SubqueryExpr represents a subquery.
    // TODO: need more descriptions.
    SubqueryExpr {
        expr: Box<Expr>,
        range: Duration,
        offset: Instant,
        timestamp: Option<i64>,
        start_or_end: ItemType, // Set when @ is used with start() or end()
        step: Duration,
    },

    NumberLiteral {
        val: f64,
    },

    StringLiteral {
        val: String,
    },

    // VectorSelector represents a Vector selection.
    VectorSelector {
        name: String,
        // offset is the actual offset that was set in the query.
        // This never changes.
        offset: Option<Instant>,
        start_or_end: ItemType, // Set when @ is used with start() or end()
        label_matchers: Vec<Matcher>,
    },

    // MatrixSelector represents a Matrix selection.
    MatrixSelector {
        // It is safe to assume that this is an VectorSelector
        // if the parser hasn't returned an error.
        vector_selector: Box<Expr>,
        range: Duration,
    },

    // Call represents a function call.
    // TODO: need more descriptions
    Call {
        func: Function,       // The function that was called.
        args: Vec<Box<Expr>>, // Arguments used in the call.
    },
}

#[derive(Debug)]
pub enum VectorMatchCardinality {
    CardOneToOne,
    CardManyToOne,
    CardOneToMany,
    CardManyToMany,
}

impl Display for VectorMatchCardinality {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            VectorMatchCardinality::CardOneToOne => write!(f, "one-to-one"),
            VectorMatchCardinality::CardManyToOne => write!(f, "many-to-one"),
            VectorMatchCardinality::CardOneToMany => write!(f, "one-to-many"),
            VectorMatchCardinality::CardManyToMany => write!(f, "many-to-many"),
        }
    }
}

// VectorMatching describes how elements from two Vectors in a binary
// operation are supposed to be matched.
#[derive(Debug)]
pub struct VectorMatching {
    // The cardinality of the two Vectors.
    card: VectorMatchCardinality,
    // MatchingLabels contains the labels which define equality of a pair of
    // elements from the Vectors.
    matching_labels: Vec<String>,
    // On includes the given label names from matching,
    // rather than excluding them.
    on: bool,
    // Include contains additional labels that should be included in
    // the result from the side with the lower cardinality.
    include: Vec<String>,
}
