use std::fmt::{self, Display};
use std::time::{Duration, Instant};

use crate::parser::{Function, Matcher, Value, ValueType};

type Pos = i32;
type ItemType = i32;

#[derive(Debug, Clone, Copy)]
pub struct PositionRange {
    pub start: i32,
    pub end: i32,
}

// Node is a generic trait for all nodes in an AST.
//
// Whenever numerous nodes are listed such as in a switch-case statement
// or a chain of function definitions (e.g. String(), PromQLExpr(), etc.) convention is
// to list them as follows:
//
//   - Statements
//   - statement types (alphabetical)
//   - ...
//   - Expressions
//   - expression types (alphabetical)
//   - ...
pub trait Node: Display {
    fn pretty(&self, _level: i32) -> String {
        String::from("")
    }
    fn pos_range(&self) -> PositionRange {
        PositionRange { start: -1, end: -1 }
    }
}

// Statement is a generic trait for all statements.
pub trait Stmt: Node {
    // stmt ensures that no other type accidentally implements the trait.
    fn promql_stmt(&self) {}
}

// EvalStmt holds an expression and information on the range it should
// be evaluated on.
pub struct EvalStmt<T: Expr> {
    expr: T, // Expression to be evaluated.

    // The time boundaries for the evaluation. If start equals end an instant
    // is evaluated.
    start: Instant,
    end: Instant,
    // Time between two evaluated instants for the range [start:end].
    interval: Duration,
    // Lookback delta to use for this evaluation.
    lookback_delta: Duration,
}

impl<T: Expr> Display for EvalStmt<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "EvalStmt {}", self.expr)
    }
}

impl<T: Expr> Node for EvalStmt<T> {
    fn pos_range(&self) -> PositionRange {
        self.expr.pos_range()
    }
}

impl<T: Expr> Stmt for EvalStmt<T> {}

// Expr is a generic trait for all expression types.
pub trait Expr: Node + Value {
    // expr ensures that no other types accidentally implement the trait.
    fn promql_expr(&self) {}
}

// UnaryExpr represents a unary operation on another expression.
// Currently unary operations are only supported for Scalars.
pub struct UnaryExpr<T: Expr> {
    op: ItemType,
    expr: T,
    start_pos: Pos,
}

impl<T: Expr> Display for UnaryExpr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UnaryExpr {}", self.expr)
    }
}

impl<T: Expr> Value for UnaryExpr<T> {
    fn vtype(&self) -> super::ValueType {
        self.expr.vtype()
    }
}

impl<T: Expr> Node for UnaryExpr<T> {
    fn pos_range(&self) -> PositionRange {
        PositionRange {
            start: self.start_pos,
            end: self.expr.pos_range().end,
        }
    }
}

impl<T: Expr> Expr for UnaryExpr<T> {}

// AggregateExpr represents an aggregation operation on a Vector.
pub struct AggregateExpr<T: Expr> {
    op: ItemType,          // The used aggregation operation.
    expr: T,               // The Vector expression over which is aggregated.
    param: T,              // Parameter used by some aggregators.
    grouping: Vec<String>, // The labels by which to group the Vector.
    without: bool,         // Whether to drop the given labels rather than keep them.
    pos_range: PositionRange,
}

impl<T: Expr> Display for AggregateExpr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AggregateExpr {}", self.expr)
    }
}

impl<T: Expr> Value for AggregateExpr<T> {
    fn vtype(&self) -> super::ValueType {
        ValueType::Vector
    }
}

impl<T: Expr> Node for AggregateExpr<T> {
    fn pos_range(&self) -> PositionRange {
        self.pos_range
    }
}

impl<T: Expr> Expr for AggregateExpr<T> {}

// BinaryExpr represents a binary expression between two child expressions.
pub struct BinaryExpr<T: Expr> {
    op: ItemType, // The operation of the expression.
    lhs: T,       // The operands on the left sides of the operator.
    rhs: T,       // The operands on the right sides of the operator.

    // The matching behavior for the operation if both operands are Vectors.
    // If they are not this field is None.
    matching: Option<VectorMatching>,

    // If a comparison operator, return 0/1 rather than filtering.
    return_bool: bool,
}

impl<T: Expr> Display for BinaryExpr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BinaryExpr {} {}", self.lhs, self.rhs)
    }
}

impl<T: Expr> Value for BinaryExpr<T> {
    fn vtype(&self) -> ValueType {
        if self.lhs.vtype() == ValueType::Scalar && self.rhs.vtype() == ValueType::Scalar {
            ValueType::Scalar
        } else {
            ValueType::Vector
        }
    }
}

impl<T: Expr> Node for BinaryExpr<T> {
    fn pos_range(&self) -> PositionRange {
        merge_ranges(&self.lhs, &self.rhs)
    }
}

impl<T: Expr> Expr for BinaryExpr<T> {}

// ParenExpr wraps an expression so it cannot be disassembled as a consequence
// of operator precedence.
pub struct ParenExpr<T: Expr> {
    expr: T,
    pos_range: PositionRange,
}

impl<T: Expr> Display for ParenExpr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParenExpr {}", self.expr)
    }
}

impl<T: Expr> Value for ParenExpr<T> {
    fn vtype(&self) -> ValueType {
        self.expr.vtype()
    }
}

impl<T: Expr> Node for ParenExpr<T> {
    fn pos_range(&self) -> PositionRange {
        self.pos_range
    }
}

impl<T: Expr> Expr for ParenExpr<T> {}

// SubqueryExpr represents a subquery.
pub struct SubqueryExpr<T: Expr> {
    expr: T,
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

    end_pos: Pos,
}

impl<T: Expr> Display for SubqueryExpr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SubqueryExpr {}", self.expr)
    }
}

impl<T: Expr> Value for SubqueryExpr<T> {
    fn vtype(&self) -> ValueType {
        ValueType::Matrix
    }
}

impl<T: Expr> Node for SubqueryExpr<T> {
    fn pos_range(&self) -> PositionRange {
        PositionRange {
            start: self.expr.pos_range().start,
            end: self.end_pos,
        }
    }
}

impl<T: Expr> Expr for SubqueryExpr<T> {}

// StepInvariantExpr represents a query which evaluates to the same result
// irrespective of the evaluation time given the raw samples from TSDB remain unchanged.
// Currently this is only used for engine optimisations and the parser does not produce this.
pub struct StepInvariantExpr<T: Expr> {
    expr: T,
}

impl<T: Expr> Display for StepInvariantExpr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StepInvariantExpr {}", self.expr)
    }
}

impl<T: Expr> Value for StepInvariantExpr<T> {
    fn vtype(&self) -> ValueType {
        self.expr.vtype()
    }
}

impl<T: Expr> Node for StepInvariantExpr<T> {
    fn pos_range(&self) -> PositionRange {
        self.expr.pos_range()
    }
}

impl<T: Expr> Expr for StepInvariantExpr<T> {}

// NumberLiteral represents a number.
pub struct NumberLiteral {
    val: f64,
    pos_range: PositionRange,
}

impl Value for NumberLiteral {
    fn vtype(&self) -> ValueType {
        ValueType::Scalar
    }
}

impl Display for NumberLiteral {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "number {}", self.val)
    }
}

impl Node for NumberLiteral {
    fn pos_range(&self) -> PositionRange {
        self.pos_range
    }
}

impl Expr for NumberLiteral {}

// StringLiteral represents a string.
pub struct StringLiteral {
    val: String,
    pos_range: PositionRange,
}

impl Display for StringLiteral {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "string {}", self.val)
    }
}

impl Value for StringLiteral {
    fn vtype(&self) -> ValueType {
        ValueType::String
    }
}

impl Node for StringLiteral {
    fn pos_range(&self) -> PositionRange {
        self.pos_range
    }
}

impl Expr for StringLiteral {}

// MatrixSelector represents a Matrix selection.
pub struct MatrixSelector<T: Expr> {
    // It is safe to assume that this is an VectorSelector
    // if the parser hasn't returned an error.
    vector_selector: T,
    range: Duration,
    end_pos: Pos,
}

impl<T: Expr> Display for MatrixSelector<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.vector_selector)
    }
}

impl<T: Expr> Value for MatrixSelector<T> {
    fn vtype(&self) -> ValueType {
        ValueType::Matrix
    }
}

impl<T: Expr> Node for MatrixSelector<T> {
    fn pos_range(&self) -> PositionRange {
        PositionRange {
            start: self.vector_selector.pos_range().start,
            end: self.end_pos,
        }
    }
}

impl<T: Expr> Expr for MatrixSelector<T> {}

// VectorSelector represents a Vector selection.
pub struct VectorSelector {
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
    pos_range: PositionRange,
}

impl Display for VectorSelector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Value for VectorSelector {
    fn vtype(&self) -> ValueType {
        ValueType::Vector
    }
}

impl Node for VectorSelector {
    fn pos_range(&self) -> PositionRange {
        self.pos_range
    }
}

impl Expr for VectorSelector {}

// Call represents a function call.
pub struct Call<T: Expr> {
    func: Function, // The function that was called.
    args: Vec<T>,   // Arguments used in the call.

    pos_range: PositionRange,
}

impl<T: Expr> Display for Call<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "call {}", self.func.name)
    }
}

impl<T: Expr> Value for Call<T> {
    fn vtype(&self) -> ValueType {
        self.func.return_type
    }
}

impl<T: Expr> Node for Call<T> {
    fn pos_range(&self) -> PositionRange {
        self.pos_range
    }
}

impl<T: Expr> Expr for Call<T> {}

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

// merge_ranges is a helper function to merge the PositionRanges of two Nodes.
// Note that the arguments must be in the same order as they
// occur in the input string.
pub fn merge_ranges<T: Node>(first: &T, last: &T) -> PositionRange {
    PositionRange {
        start: first.pos_range().start,
        end: last.pos_range().end,
    }
}

// pos_range is a helper function to calculate PositionRanges of Node Vector
pub fn pos_range<T: Expr>(exprs: Vec<T>) -> PositionRange {
    let size = exprs.len();
    if size == 0 {
        PositionRange { start: -1, end: -1 }
    } else {
        merge_ranges(&exprs[0], &exprs[size - 1])
    }
}
