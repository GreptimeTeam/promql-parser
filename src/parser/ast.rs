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
use crate::label::{Labels, Matchers};
use crate::parser::token::{self, T_END, T_START};
use crate::parser::{Function, FunctionArgs, Token, TokenType};
use std::ops::Neg;
use std::time::{Duration, SystemTime};

/// Matching Modifier, for VectorMatching of binary expr.
/// Label lists provided to matching keywords will determine how vectors are combined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorMatchModifier {
    On(Labels),
    Ignoring(Labels),
}

/// The label list provided with the group_left or group_right modifier contains
/// additional labels from the "one"-side to be included in the result metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorMatchCardinality {
    OneToOne,
    ManyToOne(Labels),
    OneToMany(Labels),
    // ManyToMany, // useless so far
}

/// Binary Expr Modifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinModifier {
    /// The matching behavior for the operation if both operands are Vectors.
    /// If they are not this field is None.
    pub card: VectorMatchCardinality,
    /// on/ignoring on labels
    pub matching: VectorMatchModifier,
    /// If a comparison operator, return 0/1 rather than filtering.
    pub return_bool: bool,
}

/// Aggregation Modifier
///
/// `without` removes the listed labels from the result vector,
/// while all other labels are preserved in the output.
/// `by` does the opposite and drops labels that are not listed in the by clause,
/// even if their label values are identical between all elements of the vector.
///
/// if empty listed labels, meaning no grouping
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggModifier {
    By(Labels),
    Without(Labels),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offset {
    Pos(Duration),
    Neg(Duration),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtModifier {
    Start,
    End,
    /// at can be earlier than UNIX_EPOCH
    At(SystemTime),
}

impl TryFrom<TokenType> for AtModifier {
    type Error = String;

    fn try_from(id: TokenType) -> Result<Self, Self::Error> {
        match id {
            T_START => Ok(AtModifier::Start),
            T_END => Ok(AtModifier::End),
            _ => Err(format!(
                "invalid @ modifier preprocessor '{}', START or END is valid.",
                token::token_display(id)
            )),
        }
    }
}

impl TryFrom<Token> for AtModifier {
    type Error = String;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
        AtModifier::try_from(token.id)
    }
}

impl TryFrom<NumberLiteral> for AtModifier {
    type Error = String;

    fn try_from(num: NumberLiteral) -> Result<Self, Self::Error> {
        AtModifier::try_from(num.val)
    }
}

impl TryFrom<Expr> for AtModifier {
    type Error = String;

    fn try_from(ex: Expr) -> Result<Self, Self::Error> {
        match ex {
            Expr::NumberLiteral(nl) => AtModifier::try_from(nl),
            _ => Err("invalid float value after @ modifier".into()),
        }
    }
}

impl TryFrom<f64> for AtModifier {
    type Error = String;

    fn try_from(secs: f64) -> Result<Self, Self::Error> {
        let err = Err(format!("timestamp out of bounds for @ modifier: {secs}"));

        if secs.is_nan() || secs.is_infinite() || secs >= f64::MAX || secs <= f64::MIN {
            return err;
        }
        let milli = (secs * 1000f64).round().abs() as u64;

        let duration = Duration::from_millis(milli);
        let mut st = Some(SystemTime::UNIX_EPOCH);
        if secs.is_sign_positive() {
            st = SystemTime::UNIX_EPOCH.checked_add(duration);
        }
        if secs.is_sign_negative() {
            st = SystemTime::UNIX_EPOCH.checked_sub(duration);
        }

        match st {
            Some(st) => Ok(Self::At(st)),
            None => err,
        }
    }
}

/// EvalStmt holds an expression and information on the range it should
/// be evaluated on.
#[derive(Debug, Clone)]
pub struct EvalStmt {
    /// Expression to be evaluated.
    pub expr: Expr,

    /// The time boundaries for the evaluation. If start equals end an instant
    /// is evaluated.
    pub start: SystemTime,
    pub end: SystemTime,
    /// Time between two evaluated instants for the range [start:end].
    pub interval: Duration,
    /// Lookback delta to use for this evaluation.
    pub lookback_delta: Duration,
}

/// <aggr-op> [without|by (<label list>)] ([parameter,] <vector expression>)
/// <aggr-op>([parameter,] <vector expression>) [without|by (<label list>)]
///
/// parameter is only required for count_values, quantile, topk and bottomk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateExpr {
    /// The used aggregation operation.
    pub op: TokenType,
    /// The Vector expression over which is aggregated.
    pub expr: Box<Expr>,
    /// Parameter used by some aggregators.
    pub param: Option<Box<Expr>>,
    pub grouping: AggModifier,
}

/// UnaryExpr will negate the expr
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnaryExpr {
    pub expr: Box<Expr>,
}

/// <vector expr> <bin-op> ignoring(<label list>) group_left(<label list>) <vector expr>
/// <vector expr> <bin-op> ignoring(<label list>) group_right(<label list>) <vector expr>
/// <vector expr> <bin-op> on(<label list>) group_left(<label list>) <vector expr>
/// <vector expr> <bin-op> on(<label list>) group_right(<label list>) <vector expr>
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryExpr {
    /// The operation of the expression.
    pub op: TokenType,
    /// The operands on the left sides of the operator.
    pub lhs: Box<Expr>,
    /// The operands on the right sides of the operator.
    pub rhs: Box<Expr>,

    pub matching: BinModifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParenExpr {
    pub expr: Box<Expr>,
}

/// <instant_query> '[' <range> ':' [<resolution>] ']' [ @ <float_literal> ] [ offset <duration> ]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubqueryExpr {
    pub expr: Box<Expr>,
    pub offset: Option<Offset>,
    pub at: Option<AtModifier>,
    pub range: Duration,
    /// Default is the global evaluation interval.
    pub step: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct NumberLiteral {
    pub val: f64,
}

impl NumberLiteral {
    pub fn new(val: f64) -> Self {
        Self { val }
    }
}

impl PartialEq for NumberLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val || self.val.is_nan() && other.val.is_nan()
    }
}

impl Eq for NumberLiteral {}

impl Neg for NumberLiteral {
    type Output = Self;

    fn neg(self) -> Self::Output {
        NumberLiteral { val: -self.val }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub val: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorSelector {
    pub name: Option<String>,
    pub label_matchers: Matchers,
    pub offset: Option<Offset>,
    pub at: Option<AtModifier>,
}

impl Neg for VectorSelector {
    type Output = UnaryExpr;

    fn neg(self) -> Self::Output {
        let ex = Expr::VectorSelector(self);
        UnaryExpr { expr: Box::new(ex) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixSelector {
    pub vector_selector: VectorSelector,
    pub range: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub func: Function,
    pub args: FunctionArgs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// SubqueryExpr represents a subquery.
    Subquery(SubqueryExpr),

    /// NumberLiteral represents a number.
    NumberLiteral(NumberLiteral),

    /// StringLiteral represents a string.
    StringLiteral(StringLiteral),

    /// VectorSelector represents a Vector selection.
    VectorSelector(VectorSelector),

    /// MatrixSelector represents a Matrix selection.
    MatrixSelector(MatrixSelector),

    /// Call represents a function call.
    Call(Call),
}

impl Expr {
    pub fn new_vector_selector(name: Option<String>, matchers: Matchers) -> Result<Self, String> {
        let vs = VectorSelector {
            name,
            offset: None,
            at: None,
            label_matchers: matchers,
        };
        Ok(Self::VectorSelector(vs))
    }

    pub fn new_unary_expr(expr: Expr) -> Result<Self, String> {
        match expr {
            Expr::StringLiteral(_) => Err("unary expression only allowed on expressions of type scalar or instant vector, got: string".into()),
            Expr::MatrixSelector(_) => Err("unary expression only allowed on expressions of type scalar or instant vector, got: range vector".into()),
            _ => Ok(-expr),
        }
    }

    pub fn new_subquery_expr(
        expr: Expr,
        range: Duration,
        step: Option<Duration>,
    ) -> Result<Self, String> {
        let se = Expr::Subquery(SubqueryExpr {
            expr: Box::new(expr),
            offset: None,
            at: None,
            range,
            step,
        });
        Ok(se)
    }

    pub fn new_paren_expr(expr: Expr) -> Result<Self, String> {
        let ex = Expr::Paren(ParenExpr {
            expr: Box::new(expr),
        });
        Ok(ex)
    }

    /// NOTE: @ and offset is not set here.
    pub fn new_matrix_selector(expr: Expr, range: Duration) -> Result<Self, String> {
        match expr {
            Expr::VectorSelector(VectorSelector {
                offset: Some(_), ..
            }) => Err("no offset modifiers allowed before range".into()),
            Expr::VectorSelector(VectorSelector { at: Some(_), .. }) => {
                Err("no @ modifiers allowed before range".into())
            }
            Expr::VectorSelector(vs) => {
                let ms = Expr::MatrixSelector(MatrixSelector {
                    vector_selector: vs,
                    range,
                });
                Ok(ms)
            }
            _ => Err("ranges only allowed for vector selectors".into()),
        }
    }

    pub fn at_expr(self, at: AtModifier) -> Result<Self, String> {
        let already_set_err = Err("@ <timestamp> may not be set multiple times".into());
        match self {
            Expr::VectorSelector(mut vs) => match vs.at {
                None => {
                    vs.at = Some(at);
                    Ok(Expr::VectorSelector(vs))
                }
                Some(_) => already_set_err,
            },
            Expr::MatrixSelector(mut ms) => match ms.vector_selector.at {
                None => {
                    ms.vector_selector.at = Some(at);
                    Ok(Expr::MatrixSelector(ms))
                }
                Some(_) => already_set_err,
            },
            Expr::Subquery(mut s) => match s.at {
                None => {
                    s.at = Some(at);
                    Ok(Expr::Subquery(s))
                }
                Some(_) => already_set_err,
            },
            _ => {
                Err("@ modifier must be preceded by an instant vector selector or range vector selector or a subquery".into())
            }
        }
    }

    /// set offset field for specified Expr, but CAN ONLY be set once.
    pub fn offset_expr(self, offset: Offset) -> Result<Self, String> {
        let already_set_err = Err("offset may not be set multiple times".into());
        match self {
            Expr::VectorSelector(mut vs) => match vs.offset {
                None => {
                    vs.offset = Some(offset);
                    Ok(Expr::VectorSelector(vs))
                }
                Some(_) => already_set_err,
            },
            Expr::MatrixSelector(mut ms) => match ms.vector_selector.offset {
                None => {
                    ms.vector_selector.offset = Some(offset);
                    Ok(Expr::MatrixSelector(ms))
                }
                Some(_) => already_set_err,
            },
            Expr::Subquery(mut s) => match s.offset {
                None => {
                    s.offset = Some(offset);
                    Ok(Expr::Subquery(s))
                }
                Some(_) => already_set_err,
            },
            _ => {
                Err("offset modifier must be preceded by an instant vector selector or range vector selector or a subquery".into())
            }
        }
    }

    pub fn new_call(func: Function, args: FunctionArgs) -> Result<Expr, String> {
        Ok(Expr::Call(Call { func, args }))
    }

    pub fn new_binary_expr(
        lhs: Expr,
        op: TokenType,
        matching: BinModifier,
        rhs: Expr,
    ) -> Result<Expr, String> {
        let ex = BinaryExpr {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            matching,
        };
        Ok(Expr::Binary(ex))
    }

    pub fn new_aggregate_expr(
        op: TokenType,
        grouping: AggModifier,
        args: FunctionArgs,
    ) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("no arguments for aggregate expression provided".into());
        }

        let mut desired_args_count = 1;
        let mut param = None;
        if token::is_aggregator_with_param(op) {
            desired_args_count = 2;
            param = Some(args.first());
        }
        if args.len() != desired_args_count {
            return Err(format!(
                "wrong number of arguments for aggregate expression provided, expected {}, got {}",
                desired_args_count,
                args.len()
            ));
        }
        let ex = AggregateExpr {
            op,
            expr: args.last(),
            param,
            grouping,
        };
        Ok(Expr::Aggregate(ex))
    }
}

impl From<String> for Expr {
    fn from(val: String) -> Self {
        Expr::StringLiteral(StringLiteral { val })
    }
}

impl From<&str> for Expr {
    fn from(s: &str) -> Self {
        Expr::StringLiteral(StringLiteral { val: s.into() })
    }
}

impl From<f64> for Expr {
    fn from(val: f64) -> Self {
        Expr::NumberLiteral(NumberLiteral { val })
    }
}

impl Neg for Expr {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Expr::NumberLiteral(nl) => Expr::NumberLiteral(-nl),
            _ => Expr::Unary(UnaryExpr {
                expr: Box::new(self),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_at_modifier() {
        let cases = vec![
            // tuple: (seconds, elapsed milliseconds before or after UNIX_EPOCH)
            (0.0, 0),
            (1000.3, 1000300),    // after UNIX_EPOCH
            (1000.9, 1000900),    // after UNIX_EPOCH
            (1000.9991, 1000999), // after UNIX_EPOCH
            (1000.9999, 1001000), // after UNIX_EPOCH
            (-1000.3, 1000300),   // before UNIX_EPOCH
            (-1000.9, 1000900),   // before UNIX_EPOCH
        ];

        for (secs, elapsed) in cases {
            match AtModifier::try_from(secs).unwrap() {
                AtModifier::At(st) => {
                    if secs.is_sign_positive() || secs == 0.0 {
                        assert_eq!(
                            elapsed,
                            st.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_millis()
                        )
                    } else if secs.is_sign_negative() {
                        assert_eq!(
                            elapsed,
                            SystemTime::UNIX_EPOCH
                                .duration_since(st)
                                .unwrap()
                                .as_millis()
                        )
                    }
                }
                _ => panic!(),
            }
        }
    }

    #[test]
    fn test_invalid_at_modifier() {
        let cases = vec![
            f64::MAX,
            f64::MIN,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];

        for secs in cases {
            assert!(AtModifier::try_from(secs).is_err())
        }
    }
}
