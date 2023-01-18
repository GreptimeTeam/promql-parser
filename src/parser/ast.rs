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
use crate::parser::{Function, Token, TokenType};

#[derive(Debug, Clone)]
pub enum AtModifier {
    Start,
    End,
    At(SystemTime),
}

impl AtModifier {
    pub fn from_float(secs: f64) -> Result<Self, String> {
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

    pub fn from_token(token: Token) -> Result<Self, String> {
        match token.id() {
            T_START => Ok(AtModifier::Start),
            T_END => Ok(AtModifier::End),
            _ => Err(format!("invalid at modifier preprocessor {}", token.val())),
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
    pub at: Option<AtModifier>,
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
    pub label_matchers: Matchers,
    pub offset: Option<Duration>,
    pub at: Option<AtModifier>,
}

#[derive(Debug, Clone)]
pub struct MatrixSelector {
    pub vector_selector: VectorSelector,
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

    pub fn step_invariant_expr(self, at_modifier: AtModifier) -> Result<Self, String> {
        let at_already_set_err = Err("@ <timestamp> may not be set multiple times".into());
        match self {
            Expr::VectorSelector(mut vs) => match vs.at {
                None => {
                    vs.at = Some(at_modifier);
                    Ok(Expr::VectorSelector(vs))
                }
                Some(_) => at_already_set_err,
            },
            Expr::MatrixSelector(mut ms) => match ms.vector_selector.at {
                None => {
                    ms.vector_selector.at = Some(at_modifier);
                    Ok(Expr::MatrixSelector(ms))
                }
                Some(_) => at_already_set_err,
            },
            Expr::Subquery(mut s) => match s.at {
                None => {
                    s.at = Some(at_modifier);
                    Ok(Expr::Subquery(s))
                }
                Some(_) => at_already_set_err,
            },
            _ => {
                Err("@ modifier must be preceded by an instant vector selector or range vector selector or a subquery".into())
            }
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

#[test]
fn test_valid_at_modifier() {
    // tuple: (seconds, elapsed based on UNIX_EPOCH)
    let cases = vec![
        (0.0, 0),
        (1000.3, 1000),  // after UNIX_EPOCH
        (1000.9, 1001),  // after UNIX_EPOCH
        (-1000.3, 1000), // before UNIX_EPOCH
        (-1000.9, 1001), // before UNIX_EPOCH
    ];

    for (secs, elapsed) in cases {
        match AtModifier::from_float(secs).unwrap() {
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
