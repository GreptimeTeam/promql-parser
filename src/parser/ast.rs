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
use std::time::{Duration, SystemTime};

use crate::label::Matchers;
use crate::parser::token::{T_END, T_START};
use crate::parser::{Function, FunctionArgs, Token, TokenType};
use crate::util::float;

pub type GroupModifier = (VectorMatching, bool);
pub type AggregateModifier = (Vec<String>, AggregateOps);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchingOps {
    On,
    Ignoring,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateOps {
    By,
    Without,
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
    At(SystemTime),
}

impl TryFrom<Token> for AtModifier {
    type Error = String;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
        match token.id() {
            T_START => Ok(AtModifier::Start),
            T_END => Ok(AtModifier::End),
            _ => Err(format!("invalid at modifier preprocessor {}", token.val())),
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

        let duration = Duration::from_secs(secs.round().abs() as u64);
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

/// <aggr-op> [without|by (<label list>)] ([parameter,] <vector expression>)
/// or
/// <aggr-op>([parameter,] <vector expression>) [without|by (<label list>)]
///
/// parameter is only required for count_values, quantile, topk and bottomk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateExpr {
    pub op: TokenType,            // The used aggregation operation.
    pub expr: Box<Expr>,          // The Vector expression over which is aggregated.
    pub param: Option<Box<Expr>>, // Parameter used by some aggregators.
    pub grouping: Vec<String>,    // The labels by which to group the Vector.
    pub how: AggregateOps,        // Whether to drop the given labels rather than keep them.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnaryExpr {
    pub op: TokenType,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryExpr {
    pub op: TokenType,  // The operation of the expression.
    pub lhs: Box<Expr>, // The operands on the left sides of the operator.
    pub rhs: Box<Expr>, // The operands on the right sides of the operator.

    // The matching behavior for the operation if both operands are Vectors.
    // If they are not this field is None.
    pub matching: VectorMatching,

    // If a comparison operator, return 0/1 rather than filtering.
    pub return_bool: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParenExpr {
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubqueryExpr {
    pub expr: Box<Expr>,
    pub offset: Option<Offset>,

    /// at modifier can be earlier than UNIX_EPOCH
    pub at: Option<AtModifier>,
    pub range: Duration,
    pub step: Duration,
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
        float::f64_equals(self.val, other.val)
    }
}

impl Eq for NumberLiteral {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub val: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorSelector {
    pub name: Option<String>,
    pub label_matchers: Matchers,
    pub offset: Option<Offset>,
    /// at modifier can be earlier than UNIX_EPOCH
    pub at: Option<AtModifier>,
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

    Subquery(SubqueryExpr),

    NumberLiteral(NumberLiteral),

    StringLiteral(StringLiteral),

    VectorSelector(VectorSelector),

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

    pub fn new_unary_expr(expr: Expr, op: &Token) -> Result<Self, String> {
        let ue = match expr {
            Expr::NumberLiteral(NumberLiteral { val }) => {
                Expr::NumberLiteral(NumberLiteral { val: -val })
            }
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

    pub fn new_number_literal(val: f64) -> Result<Self, String> {
        Ok(Expr::NumberLiteral(NumberLiteral { val }))
    }

    pub fn new_string_literal(val: String) -> Result<Self, String> {
        Ok(Expr::StringLiteral(StringLiteral { val }))
    }

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

    /// set at_modifier for specified Expr, but CAN ONLY be set once.
    pub fn step_invariant_expr(self, at_modifier: AtModifier) -> Result<Self, String> {
        let already_set_err = Err("@ <timestamp> may not be set multiple times".into());
        match self {
            Expr::VectorSelector(mut vs) => match vs.at {
                None => {
                    vs.at = Some(at_modifier);
                    Ok(Expr::VectorSelector(vs))
                }
                Some(_) => already_set_err,
            },
            Expr::MatrixSelector(mut ms) => match ms.vector_selector.at {
                None => {
                    ms.vector_selector.at = Some(at_modifier);
                    Ok(Expr::MatrixSelector(ms))
                }
                Some(_) => already_set_err,
            },
            Expr::Subquery(mut s) => match s.at {
                None => {
                    s.at = Some(at_modifier);
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
            Expr::VectorSelector(mut vs) => match vs.at {
                None => {
                    vs.offset = Some(offset);
                    Ok(Expr::VectorSelector(vs))
                }
                Some(_) => already_set_err,
            },
            Expr::MatrixSelector(mut ms) => match ms.vector_selector.at {
                None => {
                    ms.vector_selector.offset = Some(offset);
                    Ok(Expr::MatrixSelector(ms))
                }
                Some(_) => already_set_err,
            },
            Expr::Subquery(mut s) => match s.at {
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
        (matching, return_bool): GroupModifier,
        rhs: Expr,
    ) -> Result<Expr, String> {
        let ex = BinaryExpr {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            matching,
            return_bool,
        };
        Ok(Expr::Binary(ex))
    }

    pub fn new_aggregate_expr(
        token: Token,
        (grouping, how): AggregateModifier,
        args: FunctionArgs,
    ) -> Result<Expr, String> {
        if args.is_empty() {
            return Err("no arguments for aggregate expression provided".into());
        }

        let mut desired_args_count = 1;
        let mut param = None;
        if token.is_aggregator_with_param() {
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
            op: token.id(),
            expr: args.last(),
            param,
            grouping,
            how,
        };
        Ok(Expr::Aggregate(ex))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorMatching {
    pub card: VectorMatchCardinality,
    pub labels: Vec<String>,
    pub how: MatchingOps, // on, ignoring
    pub include: Vec<String>,
}

impl VectorMatching {
    pub fn new(card: VectorMatchCardinality) -> Self {
        Self {
            card,
            labels: vec![],
            how: MatchingOps::On,
            include: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_at_modifier() {
        let cases = vec![
            // tuple: (seconds, elapsed before/after UNIX_EPOCH)
            (0.0, 0),
            (1000.3, 1000),  // after UNIX_EPOCH
            (1000.9, 1001),  // after UNIX_EPOCH
            (-1000.3, 1000), // before UNIX_EPOCH
            (-1000.9, 1001), // before UNIX_EPOCH
        ];

        for (secs, elapsed) in cases {
            match AtModifier::try_from(secs).unwrap() {
                AtModifier::At(st) => {
                    if secs.is_sign_positive() || secs == 0.0 {
                        assert_eq!(
                            elapsed,
                            st.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
                        )
                    } else if secs.is_sign_negative() {
                        assert_eq!(
                            elapsed,
                            SystemTime::UNIX_EPOCH.duration_since(st).unwrap().as_secs()
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
