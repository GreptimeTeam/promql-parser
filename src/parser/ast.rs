use std::fmt::{self, Display};
use std::time::{Duration, Instant};

use crate::parser::{Function, Matcher};

type Pos = i32;
type ItemType = i32;

// Item represents a token or text string returned from the scanner.
pub struct Item {
    typ: ItemType, // The type of this Item.
    pos: Pos,      // The starting position, in bytes, of this Item in the input string.
    val: String,   // The value of this Item.
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "item {}", self.val)
    }
}

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
    AggregateExpr {
        op: ItemType,          // The used aggregation operation.
        expr: Box<Expr>,       // The Vector expression over which is aggregated.
        param: Box<Expr>,      // Parameter used by some aggregators.
        grouping: Vec<String>, // The labels by which to group the Vector.
        without: bool,         // Whether to drop the given labels rather than keep them.
    },
    UnaryExpr {
        op: ItemType,
        expr: Box<Expr>,
    },
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
    SubqueryExpr {
        expr: Box<Expr>,
        range: Duration,
        // OriginalOffset is the actual offset that was set in the query.
        // This never changes.
        original_offset: Duration,
        // Offset is the offset used during the query execution
        // which is calculated using the original offset, at modifier time,
        // eval time, and subquery offsets in the AST tree.
        offset: Duration,
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
        // OriginalOffset is the actual offset that was set in the query.
        // This never changes.
        original_offset: Duration,
        // Offset is the offset used during the query execution
        // which is calculated using the original offset, at modifier time,
        // eval time, and subquery offsets in the AST tree.
        offset: Duration,
        timestamp: Option<i64>,
        start_or_end: ItemType, // Set when @ is used with start() or end()
        label_matchers: Vec<Matcher>,
        // FIXME:
        // The unexpanded seriesSet populated at query preparation time.
        // unexpanded_series_set: storage.SeriesSet,
        // series:              []storage.Series,
    },

    // MatrixSelector represents a Matrix selection.
    MatrixSelector {
        // It is safe to assume that this is an VectorSelector
        // if the parser hasn't returned an error.
        vector_selector: Box<Expr>,
        range: Duration,
        end_pos: Pos,
    },

    // Call represents a function call.
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
