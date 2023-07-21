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

use crate::label::{Labels, Matchers, METRIC_NAME};
use crate::parser::token::{
    self, token_display, T_BOTTOMK, T_COUNT_VALUES, T_END, T_QUANTILE, T_START, T_TOPK,
};
use crate::parser::{
    Function, FunctionArgs, Prettier, Token, TokenId, TokenType, ValueType, MAX_CHARACTERS_PER_LINE,
};
use crate::util::display_duration;
use std::fmt::{self, Write};
use std::ops::Neg;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// LabelModifier acts as
///
/// # Aggregation Modifier
///
/// - Exclude means `ignoring`
/// - Include means `on`
///
/// # Vector Match Modifier
///
/// - Exclude means `without` removes the listed labels from the result vector,
/// while all other labels are preserved in the output.
/// - Include means `by` does the opposite and drops labels that are not listed in the by clause,
/// even if their label values are identical between all elements of the vector.
///
/// if empty listed labels, meaning no grouping
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelModifier {
    Include(Labels),
    Exclude(Labels),
}

impl LabelModifier {
    pub fn labels(&self) -> &Labels {
        match self {
            LabelModifier::Include(l) => l,
            LabelModifier::Exclude(l) => l,
        }
    }

    pub fn is_include(&self) -> bool {
        matches!(*self, LabelModifier::Include(_))
    }

    pub fn include(ls: Vec<&str>) -> Self {
        Self::Include(Labels::new(ls))
    }

    pub fn exclude(ls: Vec<&str>) -> Self {
        Self::Exclude(Labels::new(ls))
    }
}

/// The label list provided with the group_left or group_right modifier contains
/// additional labels from the "one"-side to be included in the result metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorMatchCardinality {
    OneToOne,
    ManyToOne(Labels),
    OneToMany(Labels),
    ManyToMany, // logical/set binary operators
}

impl VectorMatchCardinality {
    pub fn labels(&self) -> Option<&Labels> {
        match self {
            VectorMatchCardinality::ManyToOne(l) => Some(l),
            VectorMatchCardinality::OneToMany(l) => Some(l),
            VectorMatchCardinality::ManyToMany => None,
            VectorMatchCardinality::OneToOne => None,
        }
    }

    pub fn many_to_one(ls: Vec<&str>) -> Self {
        Self::ManyToOne(Labels::new(ls))
    }

    pub fn one_to_many(ls: Vec<&str>) -> Self {
        Self::OneToMany(Labels::new(ls))
    }
}

/// Binary Expr Modifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinModifier {
    /// The matching behavior for the operation if both operands are Vectors.
    /// If they are not this field is None.
    pub card: VectorMatchCardinality,

    /// on/ignoring on labels.
    /// like a + b, no match modifier is needed.
    pub matching: Option<LabelModifier>,
    /// If a comparison operator, return 0/1 rather than filtering.
    pub return_bool: bool,
}

impl fmt::Display for BinModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::from(self.bool_str());

        if let Some(matching) = &self.matching {
            match matching {
                LabelModifier::Include(ls) => write!(s, "on ({ls}) ")?,
                LabelModifier::Exclude(ls) if !ls.is_empty() => write!(s, "ignoring ({ls}) ")?,
                _ => (),
            }
        }

        match &self.card {
            VectorMatchCardinality::ManyToOne(ls) => write!(s, "group_left ({ls}) ")?,
            VectorMatchCardinality::OneToMany(ls) => write!(s, "group_right ({ls}) ")?,
            _ => (),
        }

        if s.trim().is_empty() {
            write!(f, "")
        } else {
            write!(f, " {}", s.trim_end()) // there is a leading space here
        }
    }
}

impl Default for BinModifier {
    fn default() -> Self {
        Self {
            card: VectorMatchCardinality::OneToOne,
            matching: None,
            return_bool: false,
        }
    }
}

impl BinModifier {
    pub fn with_card(mut self, card: VectorMatchCardinality) -> Self {
        self.card = card;
        self
    }

    pub fn with_matching(mut self, matching: Option<LabelModifier>) -> Self {
        self.matching = matching;
        self
    }

    pub fn with_return_bool(mut self, return_bool: bool) -> Self {
        self.return_bool = return_bool;
        self
    }

    pub fn is_labels_joint(&self) -> bool {
        matches!(
            (self.card.labels(), &self.matching),
            (Some(labels), Some(matching)) if labels.is_joint(matching.labels())
        )
    }

    pub fn intersect_labels(&self) -> Option<Vec<String>> {
        if let Some(labels) = self.card.labels() {
            if let Some(matching) = &self.matching {
                return Some(labels.intersect(matching.labels()).labels);
            }
        };
        None
    }

    pub fn is_matching_on(&self) -> bool {
        matches!(&self.matching, Some(matching) if matching.is_include())
    }

    pub fn is_matching_labels_not_empty(&self) -> bool {
        matches!(&self.matching, Some(matching) if !matching.labels().is_empty())
    }

    pub fn bool_str(&self) -> &str {
        if self.return_bool {
            "bool "
        } else {
            ""
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offset {
    Pos(Duration),
    Neg(Duration),
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Offset::Pos(dur) => write!(f, "{}", display_duration(dur)),
            Offset::Neg(dur) => write!(f, "-{}", display_duration(dur)),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtModifier {
    Start,
    End,
    /// at can be earlier than UNIX_EPOCH
    At(SystemTime),
}

impl fmt::Display for AtModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtModifier::Start => write!(f, "@ {}()", token_display(T_START)),
            AtModifier::End => write!(f, "@ {}()", token_display(T_END)),
            AtModifier::At(time) => {
                let d = time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or(Duration::ZERO); // This should not happen
                write!(f, "@ {:.3}", d.as_secs() as f64)
            }
        }
    }
}
impl TryFrom<TokenId> for AtModifier {
    type Error = String;

    fn try_from(id: TokenId) -> Result<Self, Self::Error> {
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
        AtModifier::try_from(token.id())
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
        let err_info = format!("timestamp out of bounds for @ modifier: {secs}");

        if secs.is_nan() || secs.is_infinite() || secs >= f64::MAX || secs <= f64::MIN {
            return Err(err_info);
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

        st.map(Self::At).ok_or(err_info)
    }
}

/// EvalStmt holds an expression and information on the range it should
/// be evaluated on.
#[allow(rustdoc::broken_intra_doc_links)]
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

/// Grammar:
/// ``` norust
/// <aggr-op> [without|by (<label list>)] ([parameter,] <vector expression>)
/// <aggr-op>([parameter,] <vector expression>) [without|by (<label list>)]
/// ```
///
/// parameter is only required for `count_values`, `quantile`, `topk` and `bottomk`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateExpr {
    /// The used aggregation operation.
    pub op: TokenType,
    /// The Vector expression over which is aggregated.
    pub expr: Box<Expr>,
    /// Parameter used by some aggregators.
    pub param: Option<Box<Expr>>,
    /// modifier is optional for some aggregation operators, like sum.
    pub modifier: Option<LabelModifier>,
}

impl AggregateExpr {
    fn get_op_string(&self) -> String {
        let mut s = self.op.to_string();

        if let Some(modifier) = &self.modifier {
            match modifier {
                LabelModifier::Exclude(ls) => write!(s, " without ({ls}) ").unwrap(),
                LabelModifier::Include(ls) if !ls.is_empty() => write!(s, " by ({ls}) ").unwrap(),
                _ => (),
            }
        }
        s
    }
}

impl fmt::Display for AggregateExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_op_string())?;

        write!(f, "(")?;
        if let Some(param) = &self.param {
            write!(f, "{param}, ")?;
        }
        write!(f, "{})", self.expr)?;

        Ok(())
    }
}

impl Prettier for AggregateExpr {
    fn format(&self, level: usize, max: usize) -> String {
        let mut s = format!("{}{}(\n", self.indent(level), self.get_op_string());
        if let Some(param) = &self.param {
            writeln!(s, "{},", param.pretty(level + 1, max)).unwrap();
        }
        writeln!(s, "{}", self.expr.pretty(level + 1, max)).unwrap();
        write!(s, "{})", self.indent(level)).unwrap();
        s
    }
}

/// UnaryExpr will negate the expr
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnaryExpr {
    pub expr: Box<Expr>,
}

impl fmt::Display for UnaryExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "-{}", self.expr)
    }
}

impl Prettier for UnaryExpr {
    fn pretty(&self, level: usize, max: usize) -> String {
        format!(
            "{}-{}",
            self.indent(level),
            self.expr.pretty(level, max).trim_start()
        )
    }
}

/// Grammar:
/// ``` norust
/// <vector expr> <bin-op> ignoring(<label list>) group_left(<label list>) <vector expr>
/// <vector expr> <bin-op> ignoring(<label list>) group_right(<label list>) <vector expr>
/// <vector expr> <bin-op> on(<label list>) group_left(<label list>) <vector expr>
/// <vector expr> <bin-op> on(<label list>) group_right(<label list>) <vector expr>
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryExpr {
    /// The operation of the expression.
    pub op: TokenType,
    /// The operands on the left sides of the operator.
    pub lhs: Box<Expr>,
    /// The operands on the right sides of the operator.
    pub rhs: Box<Expr>,

    pub modifier: Option<BinModifier>,
}

impl BinaryExpr {
    pub fn is_matching_on(&self) -> bool {
        matches!(&self.modifier, Some(modifier) if modifier.is_matching_on())
    }

    pub fn is_matching_labels_not_empty(&self) -> bool {
        matches!(&self.modifier, Some(modifier) if modifier.is_matching_labels_not_empty())
    }

    pub fn return_bool(&self) -> bool {
        matches!(&self.modifier, Some(modifier) if modifier.return_bool)
    }

    /// check if labels of card and matching are joint
    pub fn is_labels_joint(&self) -> bool {
        matches!(&self.modifier, Some(modifier) if modifier.is_labels_joint())
    }

    /// intersect labels of card and matching
    pub fn intersect_labels(&self) -> Option<Vec<String>> {
        self.modifier
            .as_ref()
            .and_then(|modifier| modifier.intersect_labels())
    }

    fn get_op_matching_string(&self) -> String {
        match &self.modifier {
            Some(modifier) => format!("{}{modifier}", self.op),
            None => self.op.to_string(),
        }
    }
}

impl fmt::Display for BinaryExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.lhs,
            self.get_op_matching_string(),
            self.rhs
        )
    }
}

impl Prettier for BinaryExpr {
    fn format(&self, level: usize, max: usize) -> String {
        format!(
            "{}\n{}{}\n{}",
            self.lhs.pretty(level + 1, max),
            self.indent(level),
            self.get_op_matching_string(),
            self.rhs.pretty(level + 1, max)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParenExpr {
    pub expr: Box<Expr>,
}

impl fmt::Display for ParenExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({})", self.expr)
    }
}

impl Prettier for ParenExpr {
    fn format(&self, level: usize, max: usize) -> String {
        format!(
            "{}(\n{}\n{})",
            self.indent(level),
            self.expr.pretty(level + 1, max),
            self.indent(level)
        )
    }
}

/// Grammar:
/// ```norust
/// <instant_query> '[' <range> ':' [<resolution>] ']' [ @ <float_literal> ] [ offset <duration> ]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubqueryExpr {
    pub expr: Box<Expr>,
    pub offset: Option<Offset>,
    pub at: Option<AtModifier>,
    pub range: Duration,
    /// Default is the global evaluation interval.
    pub step: Option<Duration>,
}

impl SubqueryExpr {
    fn get_time_suffix_string(&self) -> String {
        let step = match &self.step {
            Some(step) => display_duration(step),
            None => String::from(""),
        };
        let range = display_duration(&self.range);

        let mut s = format!("[{range}:{step}]");

        if let Some(at) = &self.at {
            write!(s, " {at}").unwrap();
        }

        if let Some(offset) = &self.offset {
            write!(s, " offset {offset}").unwrap();
        }
        s
    }
}

impl fmt::Display for SubqueryExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.expr, self.get_time_suffix_string())
    }
}

impl Prettier for SubqueryExpr {
    fn pretty(&self, level: usize, max: usize) -> String {
        format!(
            "{}{}",
            self.expr.pretty(level, max),
            self.get_time_suffix_string()
        )
    }
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

impl fmt::Display for NumberLiteral {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.val == f64::INFINITY {
            write!(f, "Inf")
        } else if self.val == f64::NEG_INFINITY {
            write!(f, "-Inf")
        } else if f64::is_nan(self.val) {
            write!(f, "NaN")
        } else {
            write!(f, "{}", self.val)
        }
    }
}

impl Prettier for NumberLiteral {
    fn needs_split(&self, _max: usize) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub val: String,
}

impl fmt::Display for StringLiteral {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\"", self.val)
    }
}

impl Prettier for StringLiteral {
    fn needs_split(&self, _max: usize) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorSelector {
    pub name: Option<String>,
    pub matchers: Matchers,
    pub offset: Option<Offset>,
    pub at: Option<AtModifier>,
}

impl VectorSelector {
    pub fn new(name: Option<String>, matchers: Matchers) -> Self {
        VectorSelector {
            name,
            matchers,
            offset: None,
            at: None,
        }
    }
}

impl Default for VectorSelector {
    fn default() -> Self {
        Self {
            name: None,
            matchers: Matchers::empty(),
            offset: None,
            at: None,
        }
    }
}

impl From<String> for VectorSelector {
    fn from(name: String) -> Self {
        VectorSelector {
            name: Some(name),
            offset: None,
            at: None,
            matchers: Matchers::empty(),
        }
    }
}

/// directly create an instant vector with only METRIC_NAME matcher.
///
/// # Examples
///
/// Basic usage:
///
/// ``` rust
/// use promql_parser::label::Matchers;
/// use promql_parser::parser::VectorSelector;
///
/// let vs = VectorSelector {
///     name: Some(String::from("foo")),
///     offset: None,
///     at: None,
///     matchers: Matchers::empty(),
/// };
///
/// assert_eq!(VectorSelector::from("foo"), vs);
/// ```
impl From<&str> for VectorSelector {
    fn from(name: &str) -> Self {
        VectorSelector::from(name.to_string())
    }
}

impl Neg for VectorSelector {
    type Output = UnaryExpr;

    fn neg(self) -> Self::Output {
        let ex = Expr::VectorSelector(self);
        UnaryExpr { expr: Box::new(ex) }
    }
}

impl fmt::Display for VectorSelector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{name}")?;
        }
        let matchers = &self.matchers.to_string();
        if !matchers.is_empty() {
            write!(f, "{{{matchers}}}")?;
        }
        if let Some(at) = &self.at {
            write!(f, " {at}")?;
        }
        if let Some(offset) = &self.offset {
            write!(f, " offset {offset}")?;
        }
        Ok(())
    }
}

impl Prettier for VectorSelector {
    fn needs_split(&self, _max: usize) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixSelector {
    pub vs: VectorSelector,
    pub range: Duration,
}

impl fmt::Display for MatrixSelector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = &self.vs.name {
            write!(f, "{name}")?;
        }

        let matchers = &self.vs.matchers.to_string();
        if !matchers.is_empty() {
            write!(f, "{{{matchers}}}")?;
        }

        write!(f, "[{}]", display_duration(&self.range))?;

        if let Some(at) = &self.vs.at {
            write!(f, " {at}")?;
        }

        if let Some(offset) = &self.vs.offset {
            write!(f, " offset {offset}")?;
        }

        Ok(())
    }
}

impl Prettier for MatrixSelector {
    fn needs_split(&self, _max: usize) -> bool {
        false
    }
}

/// Call represents Prometheus Function.
/// Some functions have special cases:
///
/// ## exp
///
/// exp(v instant-vector) calculates the exponential function for all elements in v.
/// Special cases are:
///
/// ```promql
/// Exp(+Inf) = +Inf
/// Exp(NaN) = NaN
/// ```
///
/// ## ln
///
/// ln(v instant-vector) calculates the natural logarithm for all elements in v.
/// Special cases are:
///
/// ```promql
/// ln(+Inf) = +Inf
/// ln(0) = -Inf
/// ln(x < 0) = NaN
/// ln(NaN) = NaN
/// ```
///
/// TODO: support more special cases of function call
///
///  - acos()
///  - acosh()
///  - asin()
///  - asinh()
///  - atan()
///  - atanh()
///  - cos()
///  - cosh()
///  - sin()
///  - sinh()
///  - tan()
///  - tanh()
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub func: Function,
    pub args: FunctionArgs,
}

impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", self.func.name, self.args)
    }
}

impl Prettier for Call {
    fn format(&self, level: usize, max: usize) -> String {
        format!(
            "{}{}(\n{}\n{})",
            self.indent(level),
            self.func.name,
            self.args.pretty(level + 1, max),
            self.indent(level)
        )
    }
}

/// Node for extending the AST. [Extension] won't be generate by this parser itself.
#[derive(Debug, Clone)]
pub struct Extension {
    pub expr: Arc<dyn ExtensionExpr>,
}

/// The interface for extending the AST with custom expression node.
pub trait ExtensionExpr: std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;

    fn name(&self) -> &str;

    fn value_type(&self) -> ValueType;

    fn children(&self) -> &[Expr];
}

impl PartialEq for Extension {
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

impl Eq for Extension {}

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

    /// Extension represents an extension expression. It is for user to attach additional
    /// informations to the AST. This parser won't generate Extension node.
    Extension(Extension),
}

impl Expr {
    pub fn new_vector_selector(name: Option<String>, matchers: Matchers) -> Result<Self, String> {
        let vs = VectorSelector::new(name, matchers);
        Ok(Self::VectorSelector(vs))
    }

    pub fn new_unary_expr(expr: Expr) -> Result<Self, String> {
        match expr {
            Expr::StringLiteral(_) => Err("unary expression only allowed on expressions of type scalar or vector, got: string".into()),
            Expr::MatrixSelector(_) => Err("unary expression only allowed on expressions of type scalar or vector, got: matrix".into()),
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
                let ms = Expr::MatrixSelector(MatrixSelector { vs, range });
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
            Expr::MatrixSelector(mut ms) => match ms.vs.at {
                None => {
                    ms.vs.at = Some(at);
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
                Err("@ modifier must be preceded by an vector selector or matrix selector or a subquery".into())
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
            Expr::MatrixSelector(mut ms) => match ms.vs.offset {
                None => {
                    ms.vs.offset = Some(offset);
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
                Err("offset modifier must be preceded by an vector selector or matrix selector or a subquery".into())
            }
        }
    }

    pub fn new_call(func: Function, args: FunctionArgs) -> Result<Expr, String> {
        Ok(Expr::Call(Call { func, args }))
    }

    pub fn new_binary_expr(
        lhs: Expr,
        op: TokenId,
        modifier: Option<BinModifier>,
        rhs: Expr,
    ) -> Result<Expr, String> {
        let ex = BinaryExpr {
            op: TokenType::new(op),
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            modifier,
        };
        Ok(Expr::Binary(ex))
    }

    pub fn new_aggregate_expr(
        op: TokenId,
        modifier: Option<LabelModifier>,
        args: FunctionArgs,
    ) -> Result<Expr, String> {
        let op = TokenType::new(op);
        if args.is_empty() {
            return Err(format!(
                "no arguments for aggregate expression '{op}' provided"
            ));
        }
        let mut desired_args_count = 1;
        let mut param = None;
        if op.is_aggregator_with_param() {
            desired_args_count = 2;
            param = args.first();
        }
        if args.len() != desired_args_count {
            return Err(format!(
                "wrong number of arguments for aggregate expression provided, expected {}, got {}",
                desired_args_count,
                args.len()
            ));
        }

        match args.last() {
            Some(expr) => Ok(Expr::Aggregate(AggregateExpr {
                op,
                expr,
                param,
                modifier,
            })),
            None => Err(
                "aggregate operation needs a single instant vector parameter, but found none"
                    .into(),
            ),
        }
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            Expr::Aggregate(_) => ValueType::Vector,
            Expr::Unary(ex) => ex.expr.value_type(),
            Expr::Binary(ex) => {
                if ex.lhs.value_type() == ValueType::Scalar
                    && ex.rhs.value_type() == ValueType::Scalar
                {
                    ValueType::Scalar
                } else {
                    ValueType::Vector
                }
            }
            Expr::Paren(ex) => ex.expr.value_type(),
            Expr::Subquery(_) => ValueType::Matrix,
            Expr::NumberLiteral(_) => ValueType::Scalar,
            Expr::StringLiteral(_) => ValueType::String,
            Expr::VectorSelector(_) => ValueType::Vector,
            Expr::MatrixSelector(_) => ValueType::Matrix,
            Expr::Call(ex) => ex.func.return_type,
            Expr::Extension(ex) => ex.expr.value_type(),
        }
    }

    /// only Some if expr is [Expr::NumberLiteral]
    pub fn scalar_value(&self) -> Option<f64> {
        match self {
            Expr::NumberLiteral(nl) => Some(nl.val),
            _ => None,
        }
    }

    pub fn prettify(&self) -> String {
        self.pretty(0, MAX_CHARACTERS_PER_LINE)
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

/// directly create an Expr::VectorSelector from instant vector
///
/// # Examples
///
/// Basic usage:
///
/// ``` rust
/// use promql_parser::label::Matchers;
/// use promql_parser::parser::{Expr, VectorSelector};
///
/// let name = String::from("foo");
/// let vs = Expr::new_vector_selector(Some(name), Matchers::empty());
///
/// assert_eq!(Expr::from(VectorSelector::from("foo")), vs.unwrap());
/// ```
impl From<VectorSelector> for Expr {
    fn from(vs: VectorSelector) -> Self {
        Expr::VectorSelector(vs)
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

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Aggregate(ex) => write!(f, "{ex}"),
            Expr::Unary(ex) => write!(f, "{ex}"),
            Expr::Binary(ex) => write!(f, "{ex}"),
            Expr::Paren(ex) => write!(f, "{ex}"),
            Expr::Subquery(ex) => write!(f, "{ex}"),
            Expr::NumberLiteral(ex) => write!(f, "{ex}"),
            Expr::StringLiteral(ex) => write!(f, "{ex}"),
            Expr::VectorSelector(ex) => write!(f, "{ex}"),
            Expr::MatrixSelector(ex) => write!(f, "{ex}"),
            Expr::Call(ex) => write!(f, "{ex}"),
            Expr::Extension(ext) => write!(f, "{ext:?}"),
        }
    }
}

impl Prettier for Expr {
    fn pretty(&self, level: usize, max: usize) -> String {
        match self {
            Expr::Aggregate(ex) => ex.pretty(level, max),
            Expr::Unary(ex) => ex.pretty(level, max),
            Expr::Binary(ex) => ex.pretty(level, max),
            Expr::Paren(ex) => ex.pretty(level, max),
            Expr::Subquery(ex) => ex.pretty(level, max),
            Expr::NumberLiteral(ex) => ex.pretty(level, max),
            Expr::StringLiteral(ex) => ex.pretty(level, max),
            Expr::VectorSelector(ex) => ex.pretty(level, max),
            Expr::MatrixSelector(ex) => ex.pretty(level, max),
            Expr::Call(ex) => ex.pretty(level, max),
            Expr::Extension(ext) => format!("{ext:?}"),
        }
    }
}

/// check_ast checks the validity of the provided AST. This includes type checking.
/// Recursively check correct typing for child nodes and raise errors in case of bad typing.
pub fn check_ast(expr: Expr) -> Result<Expr, String> {
    match expr {
        Expr::Binary(ex) => check_ast_for_binary_expr(ex),
        Expr::Aggregate(ex) => check_ast_for_aggregate_expr(ex),
        Expr::Call(ex) => check_ast_for_call(ex),
        Expr::Unary(ex) => check_ast_for_unary(ex),
        Expr::Subquery(ex) => check_ast_for_subquery(ex),
        Expr::VectorSelector(ex) => check_ast_for_vector_selector(ex),
        Expr::Paren(_) => Ok(expr),
        Expr::NumberLiteral(_) => Ok(expr),
        Expr::StringLiteral(_) => Ok(expr),
        Expr::MatrixSelector(_) => Ok(expr),
        Expr::Extension(_) => Ok(expr),
    }
}

fn expect_type(
    expected: ValueType,
    actual: Option<ValueType>,
    context: &str,
) -> Result<bool, String> {
    match actual {
        Some(actual) => {
            if actual == expected {
                Ok(true)
            } else {
                Err(format!(
                    "expected type {expected} in {context}, got {actual}"
                ))
            }
        }
        None => Err(format!("expected type {expected} in {context}, got None")),
    }
}

/// the original logic is redundant in prometheus, and the following coding blocks
/// have been optimized for readability, but all logic SHOULD be covered.
fn check_ast_for_binary_expr(mut ex: BinaryExpr) -> Result<Expr, String> {
    if !ex.op.is_operator() {
        return Err(format!(
            "binary expression does not support operator '{}'",
            ex.op
        ));
    }

    if ex.return_bool() && !ex.op.is_comparison_operator() {
        return Err("bool modifier can only be used on comparison operators".into());
    }

    if ex.op.is_comparison_operator()
        && ex.lhs.value_type() == ValueType::Scalar
        && ex.rhs.value_type() == ValueType::Scalar
        && !ex.return_bool()
    {
        return Err("comparisons between scalars must use BOOL modifier".into());
    }

    // For `on` matching, a label can only appear in one of the lists.
    // Every time series of the result vector must be uniquely identifiable.
    if ex.is_matching_on() && ex.is_labels_joint() {
        if let Some(labels) = ex.intersect_labels() {
            if let Some(label) = labels.first() {
                return Err(format!(
                    "label '{label}' must not occur in ON and GROUP clause at once"
                ));
            }
        };
    }

    if ex.op.is_set_operator() {
        if ex.lhs.value_type() == ValueType::Scalar || ex.rhs.value_type() == ValueType::Scalar {
            return Err(format!(
                "set operator '{}' not allowed in binary scalar expression",
                ex.op
            ));
        }

        if ex.lhs.value_type() == ValueType::Vector && ex.rhs.value_type() == ValueType::Vector {
            if let Some(ref modifier) = ex.modifier {
                if matches!(modifier.card, VectorMatchCardinality::OneToMany(_))
                    || matches!(modifier.card, VectorMatchCardinality::ManyToOne(_))
                {
                    return Err(format!("no grouping allowed for '{}' operation", ex.op));
                }
            };
        }

        match &mut ex.modifier {
            Some(modifier) => {
                if modifier.card == VectorMatchCardinality::OneToOne {
                    modifier.card = VectorMatchCardinality::ManyToMany;
                }
            }
            None => {
                ex.modifier =
                    Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany));
            }
        }
    }

    if ex.lhs.value_type() != ValueType::Scalar && ex.lhs.value_type() != ValueType::Vector {
        return Err("binary expression must contain only scalar and instant vector types".into());
    }
    if ex.rhs.value_type() != ValueType::Scalar && ex.rhs.value_type() != ValueType::Vector {
        return Err("binary expression must contain only scalar and instant vector types".into());
    }

    if (ex.lhs.value_type() != ValueType::Vector || ex.rhs.value_type() != ValueType::Vector)
        && ex.is_matching_labels_not_empty()
    {
        return Err("vector matching only allowed between vectors".into());
    }

    Ok(Expr::Binary(ex))
}

fn check_ast_for_aggregate_expr(ex: AggregateExpr) -> Result<Expr, String> {
    if !ex.op.is_aggregator() {
        return Err(format!(
            "aggregation operator expected in aggregation expression but got '{}'",
            ex.op
        ));
    }

    expect_type(
        ValueType::Vector,
        Some(ex.expr.value_type()),
        "aggregation expression",
    )?;

    if matches!(ex.op.id(), T_TOPK | T_BOTTOMK | T_QUANTILE) {
        expect_type(
            ValueType::Scalar,
            ex.param.as_ref().map(|ex| ex.value_type()),
            "aggregation expression",
        )?;
    }

    if ex.op.id() == T_COUNT_VALUES {
        expect_type(
            ValueType::String,
            ex.param.as_ref().map(|ex| ex.value_type()),
            "aggregation expression",
        )?;
    }

    Ok(Expr::Aggregate(ex))
}

fn check_ast_for_call(ex: Call) -> Result<Expr, String> {
    let expected_args_len = ex.func.arg_types.len();
    let name = ex.func.name;
    let actual_args_len = ex.args.len();

    if ex.func.variadic {
        let expected_args_len_without_default = expected_args_len - 1;
        if expected_args_len_without_default > actual_args_len {
            return Err(format!(
                "expected at least {expected_args_len_without_default} argument(s) in call to '{name}', got {actual_args_len}"
            ));
        }

        // `label_join` do not have a maximum arguments threshold.
        // this hard code SHOULD be careful if new functions are supported by Prometheus.
        if actual_args_len > expected_args_len && name.ne("label_join") {
            return Err(format!(
                "expected at most {expected_args_len} argument(s) in call to '{name}', got {actual_args_len}"
            ));
        }
    }

    if !ex.func.variadic && expected_args_len != actual_args_len {
        return Err(format!(
            "expected {expected_args_len} argument(s) in call to '{name}', got {actual_args_len}"
        ));
    }

    // special cases from https://prometheus.io/docs/prometheus/latest/querying/functions
    if name.eq("exp") {
        if let Some(val) = ex.args.first().and_then(|ex| ex.scalar_value()) {
            if val.is_nan() || val.is_infinite() {
                return Ok(Expr::Call(ex));
            }
        }
    } else if name.eq("ln") || name.eq("log2") || name.eq("log10") {
        if let Some(val) = ex.args.first().and_then(|ex| ex.scalar_value()) {
            if val.is_nan() || val.is_infinite() || val <= 0.0 {
                return Ok(Expr::Call(ex));
            }
        }
    }

    for (mut idx, actual_arg) in ex.args.args.iter().enumerate() {
        // this only happens when function args are variadic
        if idx >= ex.func.arg_types.len() {
            idx = ex.func.arg_types.len() - 1;
        }

        expect_type(
            ex.func.arg_types[idx],
            Some(actual_arg.value_type()),
            &format!("call to function '{name}'"),
        )?;
    }

    Ok(Expr::Call(ex))
}

fn check_ast_for_unary(ex: UnaryExpr) -> Result<Expr, String> {
    let value_type = ex.expr.value_type();
    if value_type != ValueType::Scalar && value_type != ValueType::Vector {
        return Err(format!(
            "unary expression only allowed on expressions of type scalar or vector, got {value_type}"
        ));
    }

    Ok(Expr::Unary(ex))
}

fn check_ast_for_subquery(ex: SubqueryExpr) -> Result<Expr, String> {
    let value_type = ex.expr.value_type();
    if value_type != ValueType::Vector {
        return Err(format!(
            "subquery is only allowed on vector, got {value_type} instead"
        ));
    }

    Ok(Expr::Subquery(ex))
}

fn check_ast_for_vector_selector(ex: VectorSelector) -> Result<Expr, String> {
    match ex.name {
        Some(ref name) => match ex.matchers.find_matcher_value(METRIC_NAME) {
            Some(val) => Err(format!(
                "metric name must not be set twice: '{}' or '{}'",
                name, val
            )),
            None => Ok(Expr::VectorSelector(ex)),
        },
        None if ex.matchers.is_empty_matchers() => {
            // When name is None, a vector selector must contain at least one non-empty matcher
            // to prevent implicit selection of all metrics (e.g. by a typo).
            Err("vector selector must contain at least one non-empty matcher".into())
        }
        _ => Ok(Expr::VectorSelector(ex)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::label::{MatchOp, Matcher, Matchers};

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

        assert_eq!(
            AtModifier::try_from(Expr::from(1.0)),
            AtModifier::try_from(1.0),
        );
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

        assert_eq!(
            AtModifier::try_from(token::T_ADD),
            Err("invalid @ modifier preprocessor '+', START or END is valid.".into())
        );

        assert_eq!(
            AtModifier::try_from(Expr::from("string literal")),
            Err("invalid float value after @ modifier".into())
        );
    }

    #[test]
    fn test_binary_labels() {
        assert_eq!(
            &Labels::new(vec!["foo", "bar"]),
            LabelModifier::Include(Labels::new(vec!["foo", "bar"])).labels()
        );

        assert_eq!(
            &Labels::new(vec!["foo", "bar"]),
            LabelModifier::Exclude(Labels::new(vec!["foo", "bar"])).labels()
        );

        assert_eq!(
            &Labels::new(vec!["foo", "bar"]),
            VectorMatchCardinality::OneToMany(Labels::new(vec!["foo", "bar"]))
                .labels()
                .unwrap()
        );

        assert_eq!(
            &Labels::new(vec!["foo", "bar"]),
            VectorMatchCardinality::ManyToOne(Labels::new(vec!["foo", "bar"]))
                .labels()
                .unwrap()
        );

        assert_eq!(VectorMatchCardinality::OneToOne.labels(), None);
        assert_eq!(VectorMatchCardinality::ManyToMany.labels(), None);
    }

    #[test]
    fn test_neg() {
        assert_eq!(
            -VectorSelector::from("foo"),
            UnaryExpr {
                expr: Box::new(Expr::from(VectorSelector::from("foo")))
            }
        )
    }

    #[test]
    fn test_scalar_value() {
        assert_eq!(Some(1.0), Expr::from(1.0).scalar_value());
        assert_eq!(None, Expr::from("1.0").scalar_value());
    }

    #[test]
    fn test_at_expr() {
        assert_eq!(
            "@ <timestamp> may not be set multiple times",
            Expr::from(VectorSelector::from("foo"))
                .at_expr(AtModifier::try_from(1.0).unwrap())
                .and_then(|ex| ex.at_expr(AtModifier::try_from(1.0).unwrap()))
                .unwrap_err()
        );

        assert_eq!(
            "@ <timestamp> may not be set multiple times",
            Expr::new_matrix_selector(
                Expr::from(VectorSelector::from("foo")),
                Duration::from_secs(1),
            )
            .and_then(|ex| ex.at_expr(AtModifier::try_from(1.0).unwrap()))
            .and_then(|ex| ex.at_expr(AtModifier::try_from(1.0).unwrap()))
            .unwrap_err()
        );

        assert_eq!(
            "@ <timestamp> may not be set multiple times",
            Expr::new_subquery_expr(
                Expr::from(VectorSelector::from("foo")),
                Duration::from_secs(1),
                None,
            )
            .and_then(|ex| ex.at_expr(AtModifier::try_from(1.0).unwrap()))
            .and_then(|ex| ex.at_expr(AtModifier::try_from(1.0).unwrap()))
            .unwrap_err()
        )
    }

    #[test]
    fn test_offset_expr() {
        assert_eq!(
            "offset may not be set multiple times",
            Expr::from(VectorSelector::from("foo"))
                .offset_expr(Offset::Pos(Duration::from_secs(1000)))
                .and_then(|ex| ex.offset_expr(Offset::Pos(Duration::from_secs(1000))))
                .unwrap_err()
        );

        assert_eq!(
            "offset may not be set multiple times",
            Expr::new_matrix_selector(
                Expr::from(VectorSelector::from("foo")),
                Duration::from_secs(1),
            )
            .and_then(|ex| ex.offset_expr(Offset::Pos(Duration::from_secs(1000))))
            .and_then(|ex| ex.offset_expr(Offset::Pos(Duration::from_secs(1000))))
            .unwrap_err()
        );

        assert_eq!(
            "offset may not be set multiple times",
            Expr::new_subquery_expr(
                Expr::from(VectorSelector::from("foo")),
                Duration::from_secs(1),
                None,
            )
            .and_then(|ex| ex.offset_expr(Offset::Pos(Duration::from_secs(1000))))
            .and_then(|ex| ex.offset_expr(Offset::Pos(Duration::from_secs(1000))))
            .unwrap_err()
        );
    }

    #[test]
    fn test_expr_to_string() {
        let mut cases = vec![
            ("1", "1"),
            ("- 1", "-1"),
            ("+ 1", "1"),
            ("Inf", "Inf"),
            ("inf", "Inf"),
            ("+Inf", "Inf"),
            ("- Inf", "-Inf"),
            (".5", "0.5"),
            ("5.", "5"),
            ("123.4567", "123.4567"),
            ("5e-3", "0.005"),
            ("5e3", "5000"),
            ("0xc", "12"),
            ("0755", "493"),
            ("08", "8"),
            ("+5.5e-3", "0.0055"),
            ("-0755", "-493"),
            ("NaN", "NaN"),
            ("NAN", "NaN"),
            ("- 1^2", "-1 ^ 2"),
            ("+1 + -2 * 1", "1 + -2 * 1"),
            ("1 + 2/(3*1)", "1 + 2 / (3 * 1)"),
            ("foo*sum", "foo * sum"),
            ("foo * on(test,blub) bar", "foo * on (test, blub) bar"),
            (
                r#"up{job="hi", instance="in"} offset 5m @ 100"#,
                r#"up{instance="in",job="hi"} @ 100.000 offset 5m"#,
            ),
            (
                r#"up{job="hi", instance="in"}"#,
                r#"up{instance="in",job="hi"}"#,
            ),
            ("sum (up) by (job,instance)", "sum by (job, instance) (up)"),
            (
                "foo / on(test,blub) group_left(bar) bar",
                "foo / on (test, blub) group_left (bar) bar",
            ),
            (
                "foo / on(test,blub) group_right(bar) bar",
                "foo / on (test, blub) group_right (bar) bar",
            ),
            (
                r#"foo{a="b",foo!="bar",test=~"test",bar!~"baz"}"#,
                r#"foo{a="b",bar!~"baz",foo!="bar",test=~"test"}"#,
            ),
            (
                r#"{__name__=~"foo.+",__name__=~".*bar"}"#,
                r#"{__name__=~".*bar",__name__=~"foo.+"}"#,
            ),
            (
                r#"test{a="b"}[5y] OFFSET 3d"#,
                r#"test{a="b"}[5y] offset 3d"#,
            ),
            (
                r#"{a="b"}[5y] OFFSET 3d"#,
                r#"{a="b"}[5y] offset 3d"#,
            ),
            (
                "sum(some_metric) without(and, by, avg, count, alert, annotations)",
                "sum without (and, by, avg, count, alert, annotations) (some_metric)",
            ),
            (
                r#"floor(some_metric{foo!="bar"})"#,
                r#"floor(some_metric{foo!="bar"})"#,
            ),
            (
                "sum(rate(http_request_duration_seconds[10m])) / count(rate(http_request_duration_seconds[10m]))",
                "sum(rate(http_request_duration_seconds[10m])) / count(rate(http_request_duration_seconds[10m]))",
            ),
            ("rate(some_metric[5m])", "rate(some_metric[5m])"),
            ("round(some_metric,5)", "round(some_metric, 5)"),
            (
                r#"absent(sum(nonexistent{job="myjob"}))"#,
                r#"absent(sum(nonexistent{job="myjob"}))"#,
            ),
            (
                "histogram_quantile(0.9,rate(http_request_duration_seconds_bucket[10m]))",
                "histogram_quantile(0.9, rate(http_request_duration_seconds_bucket[10m]))",
            ),
            (
                "histogram_quantile(0.9,sum(rate(http_request_duration_seconds_bucket[10m])) by(job,le))",
                "histogram_quantile(0.9, sum by (job, le) (rate(http_request_duration_seconds_bucket[10m])))",
            ),
            (
                r#"label_join(up{job="api-server",src1="a",src2="b",src3="c"}, "foo", ",", "src1", "src2", "src3")"#,
                r#"label_join(up{job="api-server",src1="a",src2="b",src3="c"}, "foo", ",", "src1", "src2", "src3")"#,
            ),
            (
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:])[4m:3s] @ 100"#,
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:])[4m:3s] @ 100.000"#,
            ),
            (
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:])[4m:3s]"#,
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:])[4m:3s]"#,
            ),
            (
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:] offset 4m)[4m:3s]"#,
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:] offset 4m)[4m:3s]"#,
            ),
            ("some_metric OFFSET 1m [10m:5s]", "some_metric offset 1m[10m:5s]"),
            ("some_metric @123 [10m:5s]", "some_metric @ 123.000[10m:5s]")
        ];

        // the following cases are from https://github.com/prometheus/prometheus/blob/main/promql/parser/printer_test.go
        let mut cases1 = vec![
            (
                r#"sum by() (task:errors:rate10s{job="s"})"#,
                r#"sum(task:errors:rate10s{job="s"})"#,
            ),
            (
                r#"sum by(code) (task:errors:rate10s{job="s"})"#,
                r#"sum by (code) (task:errors:rate10s{job="s"})"#,
            ),
            (
                r#"sum without() (task:errors:rate10s{job="s"})"#,
                r#"sum without () (task:errors:rate10s{job="s"})"#,
            ),
            (
                r#"sum without(instance) (task:errors:rate10s{job="s"})"#,
                r#"sum without (instance) (task:errors:rate10s{job="s"})"#,
            ),
            (
                r#"topk(5, task:errors:rate10s{job="s"})"#,
                r#"topk(5, task:errors:rate10s{job="s"})"#,
            ),
            (
                r#"count_values("value", task:errors:rate10s{job="s"})"#,
                r#"count_values("value", task:errors:rate10s{job="s"})"#,
            ),
            ("a - on() c", "a - on () c"),
            ("a - on(b) c", "a - on (b) c"),
            ("a - on(b) group_left(x) c", "a - on (b) group_left (x) c"),
            (
                "a - on(b) group_left(x, y) c",
                "a - on (b) group_left (x, y) c",
            ),
            ("a - on(b) group_left c", "a - on (b) group_left () c"),
            ("a - ignoring(b) c", "a - ignoring (b) c"),
            ("a - ignoring() c", "a - c"),
            ("up > bool 0", "up > bool 0"),
            ("a offset 1m", "a offset 1m"),
            ("a offset -7m", "a offset -7m"),
            (r#"a{c="d"}[5m] offset 1m"#, r#"a{c="d"}[5m] offset 1m"#),
            ("a[5m] offset 1m", "a[5m] offset 1m"),
            ("a[12m] offset -3m", "a[12m] offset -3m"),
            ("a[1h:5m] offset 1m", "a[1h:5m] offset 1m"),
            (r#"{__name__="a"}"#, r#"{__name__="a"}"#),
            (r#"a{b!="c"}[1m]"#, r#"a{b!="c"}[1m]"#),
            (r#"a{b=~"c"}[1m]"#, r#"a{b=~"c"}[1m]"#),
            (r#"a{b!~"c"}[1m]"#, r#"a{b!~"c"}[1m]"#),
            ("a @ 10", "a @ 10.000"),
            ("a[1m] @ 10", "a[1m] @ 10.000"),
            ("a @ start()", "a @ start()"),
            ("a @ end()", "a @ end()"),
            ("a[1m] @ start()", "a[1m] @ start()"),
            ("a[1m] @ end()", "a[1m] @ end()"),
        ];

        cases.append(&mut cases1);
        for (input, expected) in cases {
            let expr = crate::parser::parse(input).unwrap();
            assert_eq!(expected, expr.to_string())
        }
    }

    #[test]
    fn test_vector_selector_to_string() {
        let cases = vec![
            (VectorSelector::default(), ""),
            (VectorSelector::from("foobar"), "foobar"),
            (
                {
                    let name = Some(String::from("foobar"));
                    let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "a", "x"));
                    VectorSelector::new(name, matchers)
                },
                r#"foobar{a="x"}"#,
            ),
            (
                {
                    let matchers = Matchers::new(vec![
                        Matcher::new(MatchOp::Equal, "a", "x"),
                        Matcher::new(MatchOp::Equal, "b", "y"),
                    ]);
                    VectorSelector::new(None, matchers)
                },
                r#"{a="x",b="y"}"#,
            ),
            (
                {
                    let matchers =
                        Matchers::one(Matcher::new(MatchOp::Equal, METRIC_NAME, "foobar"));
                    VectorSelector::new(None, matchers)
                },
                r#"{__name__="foobar"}"#,
            ),
        ];

        for (vs, expect) in cases {
            assert_eq!(expect, vs.to_string())
        }
    }

    #[test]
    fn test_aggregate_expr_pretty() {
        let cases = vec![
            ("sum(foo)", "sum(foo)"),
            (
                r#"sum by() (task:errors:rate10s{job="s"})"#,
                r#"sum(
  task:errors:rate10s{job="s"}
)"#,
            ),
            (
                r#"sum without(job,foo) (task:errors:rate10s{job="s"})"#,
                r#"sum without (job, foo) (
  task:errors:rate10s{job="s"}
)"#,
            ),
            (
                r#"sum(task:errors:rate10s{job="s"}) without(job,foo)"#,
                r#"sum without (job, foo) (
  task:errors:rate10s{job="s"}
)"#,
            ),
            (
                r#"sum by(job,foo) (task:errors:rate10s{job="s"})"#,
                r#"sum by (job, foo) (
  task:errors:rate10s{job="s"}
)"#,
            ),
            (
                r#"sum (task:errors:rate10s{job="s"}) by(job,foo)"#,
                r#"sum by (job, foo) (
  task:errors:rate10s{job="s"}
)"#,
            ),
            (
                r#"topk(10, ask:errors:rate10s{job="s"})"#,
                r#"topk(
  10,
  ask:errors:rate10s{job="s"}
)"#,
            ),
            (
                r#"sum by(job,foo) (sum by(job,foo) (task:errors:rate10s{job="s"}))"#,
                r#"sum by (job, foo) (
  sum by (job, foo) (
    task:errors:rate10s{job="s"}
  )
)"#,
            ),
            (
                r#"sum by(job,foo) (sum by(job,foo) (sum by(job,foo) (task:errors:rate10s{job="s"})))"#,
                r#"sum by (job, foo) (
  sum by (job, foo) (
    sum by (job, foo) (
      task:errors:rate10s{job="s"}
    )
  )
)"#,
            ),
            (
                r#"sum by(job,foo)
(sum by(job,foo) (task:errors:rate10s{job="s"}))"#,
                r#"sum by (job, foo) (
  sum by (job, foo) (
    task:errors:rate10s{job="s"}
  )
)"#,
            ),
            (
                r#"sum by(job,foo)
(sum(task:errors:rate10s{job="s"}) without(job,foo))"#,
                r#"sum by (job, foo) (
  sum without (job, foo) (
    task:errors:rate10s{job="s"}
  )
)"#,
            ),
            (
                r#"sum by(job,foo) # Comment 1.
(sum by(job,foo) ( # Comment 2.
task:errors:rate10s{job="s"}))"#,
                r#"sum by (job, foo) (
  sum by (job, foo) (
    task:errors:rate10s{job="s"}
  )
)"#,
            ),
        ];

        for (input, expect) in cases {
            let expr = crate::parser::parse(&input);
            assert_eq!(expect, expr.unwrap().pretty(0, 10));
        }
    }

    #[test]
    fn test_binary_expr_pretty() {
        let cases = vec![
            ("a+b", "a + b"),
            (
                "a == bool 1",
                "  a
== bool
  1",
            ),
            (
                "a == 1024000",
                "  a
==
  1024000",
            ),
            (
                "a + ignoring(job) b",
                "  a
+ ignoring (job)
  b",
            ),
            (
                "foo_1 + foo_2",
                "  foo_1
+
  foo_2",
            ),
            (
                "foo_1 + foo_2 + foo_3",
                "    foo_1
  +
    foo_2
+
  foo_3",
            ),
            (
                "foo + baar + foo_3",
                "  foo + baar
+
  foo_3",
            ),
            (
                "foo_1 + foo_2 + foo_3 + foo_4",
                "      foo_1
    +
      foo_2
  +
    foo_3
+
  foo_4",
            ),
            (
                "foo_1 + ignoring(foo) foo_2 + ignoring(job) group_left foo_3 + on(instance) group_right foo_4",

                 "      foo_1
    + ignoring (foo)
      foo_2
  + ignoring (job) group_left ()
    foo_3
+ on (instance) group_right ()
  foo_4",
            ),
        ];

        for (input, expect) in cases {
            let expr = crate::parser::parse(&input);
            assert_eq!(expect, expr.unwrap().pretty(0, 10));
        }
    }

    #[test]
    fn test_call_expr_pretty() {
        let cases = vec![
            (
                "rate(foo[1m])",
                "rate(
  foo[1m]
)",
            ),
            (
                "sum_over_time(foo[1m])",
                "sum_over_time(
  foo[1m]
)",
            ),
            (
                "rate(long_vector_selector[10m:1m] @ start() offset 1m)",
                "rate(
  long_vector_selector[10m:1m] @ start() offset 1m
)",
            ),
            (
                "histogram_quantile(0.9, rate(foo[1m]))",
                "histogram_quantile(
  0.9,
  rate(
    foo[1m]
  )
)",
            ),
            (
                "histogram_quantile(0.9, rate(foo[1m] @ start()))",
                "histogram_quantile(
  0.9,
  rate(
    foo[1m] @ start()
  )
)",
            ),
            (
                "max_over_time(rate(demo_api_request_duration_seconds_count[1m])[1m:] @ start() offset 1m)",
                "max_over_time(
  rate(
    demo_api_request_duration_seconds_count[1m]
  )[1m:] @ start() offset 1m
)",
            ),
            (
                r#"label_replace(up{job="api-server",service="a:c"}, "foo", "$1", "service", "(.*):.*")"#,
                r#"label_replace(
  up{job="api-server",service="a:c"},
  "foo",
  "$1",
  "service",
  "(.*):.*"
)"#,
            ),
            (
                r#"label_replace(label_replace(up{job="api-server",service="a:c"}, "foo", "$1", "service", "(.*):.*"), "foo", "$1", "service", "(.*):.*")"#,
                r#"label_replace(
  label_replace(
    up{job="api-server",service="a:c"},
    "foo",
    "$1",
    "service",
    "(.*):.*"
  ),
  "foo",
  "$1",
  "service",
  "(.*):.*"
)"#,
            ),
        ];

        for (input, expect) in cases {
            let expr = crate::parser::parse(&input);
            assert_eq!(expect, expr.unwrap().pretty(0, 10));
        }
    }

    #[test]
    fn test_paren_expr_pretty() {
        let cases = vec![
            ("(foo)", "(foo)"),
            (
                "(_foo_long_)",
                "(
  _foo_long_
)",
            ),
            (
                "((foo_long))",
                "(
  (foo_long)
)",
            ),
            (
                "((_foo_long_))",
                "(
  (
    _foo_long_
  )
)",
            ),
            (
                "(((foo_long)))",
                "(
  (
    (foo_long)
  )
)",
            ),
            ("(1 + 2)", "(1 + 2)"),
            (
                "(foo + bar)",
                "(
  foo + bar
)",
            ),
            (
                "(foo_long + bar_long)",
                "(
    foo_long
  +
    bar_long
)",
            ),
            (
                "(foo_long + bar_long + bar_2_long)",
                "(
      foo_long
    +
      bar_long
  +
    bar_2_long
)",
            ),
            (
                "((foo_long + bar_long) + bar_2_long)",
                "(
    (
        foo_long
      +
        bar_long
    )
  +
    bar_2_long
)",
            ),
            (
                "(1111 + 2222)",
                "(
    1111
  +
    2222
)",
            ),
            (
                "(sum_over_time(foo[1m]))",
                "(
  sum_over_time(
    foo[1m]
  )
)",
            ),
            (
                r#"(label_replace(up{job="api-server",service="a:c"}, "foo", "$1", "service", "(.*):.*"))"#,
                r#"(
  label_replace(
    up{job="api-server",service="a:c"},
    "foo",
    "$1",
    "service",
    "(.*):.*"
  )
)"#,
            ),
            (
                r#"(label_replace(label_replace(up{job="api-server",service="a:c"}, "foo", "$1", "service", "(.*):.*"), "foo", "$1", "service", "(.*):.*"))"#,
                r#"(
  label_replace(
    label_replace(
      up{job="api-server",service="a:c"},
      "foo",
      "$1",
      "service",
      "(.*):.*"
    ),
    "foo",
    "$1",
    "service",
    "(.*):.*"
  )
)"#,
            ),
            (
                r#"(label_replace(label_replace((up{job="api-server",service="a:c"}), "foo", "$1", "service", "(.*):.*"), "foo", "$1", "service", "(.*):.*"))"#,
                r#"(
  label_replace(
    label_replace(
      (
        up{job="api-server",service="a:c"}
      ),
      "foo",
      "$1",
      "service",
      "(.*):.*"
    ),
    "foo",
    "$1",
    "service",
    "(.*):.*"
  )
)"#,
            ),
        ];

        for (input, expect) in cases {
            let expr = crate::parser::parse(&input);
            assert_eq!(expect, expr.unwrap().pretty(0, 10));
        }
    }

    #[test]
    fn test_unary_expr_pretty() {
        let cases = vec![
            ("-1", "-1"),
            ("-vector_selector", "-vector_selector"),
            (
                "(-vector_selector)",
                "(
  -vector_selector
)",
            ),
            (
                "-histogram_quantile(0.9,rate(foo[1m]))",
                "-histogram_quantile(
  0.9,
  rate(
    foo[1m]
  )
)",
            ),
            (
                "-histogram_quantile(0.99, sum by (le) (rate(foo[1m])))",
                "-histogram_quantile(
  0.99,
  sum by (le) (
    rate(
      foo[1m]
    )
  )
)",
            ),
            (
                "-histogram_quantile(0.9, -rate(foo[1m] @ start()))",
                "-histogram_quantile(
  0.9,
  -rate(
    foo[1m] @ start()
  )
)",
            ),
            (
                "(-histogram_quantile(0.9, -rate(foo[1m] @ start())))",
                "(
  -histogram_quantile(
    0.9,
    -rate(
      foo[1m] @ start()
    )
  )
)",
            ),
        ];

        for (input, expect) in cases {
            let expr = crate::parser::parse(&input);
            assert_eq!(expect, expr.unwrap().pretty(0, 10));
        }
    }

    #[test]
    fn test_expr_pretty() {
        // Following queries have been taken from https://monitoring.mixins.dev/
        let cases = vec![
            (
                r#"(node_filesystem_avail_bytes{job="node",fstype!=""} / node_filesystem_size_bytes{job="node",fstype!=""} * 100 < 40 and predict_linear(node_filesystem_avail_bytes{job="node",fstype!=""}[6h], 24*60*60) < 0 and node_filesystem_readonly{job="node",fstype!=""} == 0)"#,
                r#"(
            node_filesystem_avail_bytes{fstype!="",job="node"}
          /
            node_filesystem_size_bytes{fstype!="",job="node"}
        *
          100
      <
        40
    and
        predict_linear(
          node_filesystem_avail_bytes{fstype!="",job="node"}[6h],
            24 * 60
          *
            60
        )
      <
        0
  and
      node_filesystem_readonly{fstype!="",job="node"}
    ==
      0
)"#,
            ),
            (
                r#"(node_filesystem_avail_bytes{job="node",fstype!=""} / node_filesystem_size_bytes{job="node",fstype!=""} * 100 < 20 and predict_linear(node_filesystem_avail_bytes{job="node",fstype!=""}[6h], 4*60*60) < 0 and node_filesystem_readonly{job="node",fstype!=""} == 0)"#,
                r#"(
            node_filesystem_avail_bytes{fstype!="",job="node"}
          /
            node_filesystem_size_bytes{fstype!="",job="node"}
        *
          100
      <
        20
    and
        predict_linear(
          node_filesystem_avail_bytes{fstype!="",job="node"}[6h],
            4 * 60
          *
            60
        )
      <
        0
  and
      node_filesystem_readonly{fstype!="",job="node"}
    ==
      0
)"#,
            ),
            (
                r#"(node_timex_offset_seconds > 0.05 and deriv(node_timex_offset_seconds[5m]) >= 0) or (node_timex_offset_seconds < -0.05 and deriv(node_timex_offset_seconds[5m]) <= 0)"#,
                r#"  (
        node_timex_offset_seconds
      >
        0.05
    and
        deriv(
          node_timex_offset_seconds[5m]
        )
      >=
        0
  )
or
  (
        node_timex_offset_seconds
      <
        -0.05
    and
        deriv(
          node_timex_offset_seconds[5m]
        )
      <=
        0
  )"#,
            ),
            (
                r#"1 - ((node_memory_MemAvailable_bytes{job="node"} or (node_memory_Buffers_bytes{job="node"} + node_memory_Cached_bytes{job="node"} + node_memory_MemFree_bytes{job="node"} + node_memory_Slab_bytes{job="node"}) ) / node_memory_MemTotal_bytes{job="node"})"#,
                r#"  1
-
  (
      (
          node_memory_MemAvailable_bytes{job="node"}
        or
          (
                  node_memory_Buffers_bytes{job="node"}
                +
                  node_memory_Cached_bytes{job="node"}
              +
                node_memory_MemFree_bytes{job="node"}
            +
              node_memory_Slab_bytes{job="node"}
          )
      )
    /
      node_memory_MemTotal_bytes{job="node"}
  )"#,
            ),
            (
                r#"min by (job, integration) (rate(alertmanager_notifications_failed_total{job="alertmanager", integration=~".*"}[5m]) / rate(alertmanager_notifications_total{job="alertmanager", integration="~.*"}[5m])) > 0.01"#,
                r#"  min by (job, integration) (
      rate(
        alertmanager_notifications_failed_total{integration=~".*",job="alertmanager"}[5m]
      )
    /
      rate(
        alertmanager_notifications_total{integration="~.*",job="alertmanager"}[5m]
      )
  )
>
  0.01"#,
            ),
            (
                r#"(count by (job) (changes(process_start_time_seconds{job="alertmanager"}[10m]) > 4) / count by (job) (up{job="alertmanager"})) >= 0.5"#,
                r#"  (
      count by (job) (
          changes(
            process_start_time_seconds{job="alertmanager"}[10m]
          )
        >
          4
      )
    /
      count by (job) (
        up{job="alertmanager"}
      )
  )
>=
  0.5"#,
            ),
        ];

        for (input, expect) in cases {
            let expr = crate::parser::parse(&input);
            assert_eq!(expect, expr.unwrap().pretty(0, 10));
        }
    }

    #[test]
    fn test_step_invariant_pretty() {
        let cases = vec![
            ("a @ 1", "a @ 1.000"),
            ("a @ start()", "a @ start()"),
            ("vector_selector @ start()", "vector_selector @ start()"),
        ];

        for (input, expect) in cases {
            let expr = crate::parser::parse(&input);
            assert_eq!(expect, expr.unwrap().pretty(0, 10));
        }
    }
}
