// Copyright 2019 The Prometheus Authors
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

// Diff with promql's generated_parser.y
//
// - no empty rule
// - no series descriptions rule

%token EQL
BLANK
COLON
COMMA
COMMENT
DURATION
EOF
ERROR
IDENTIFIER
LEFT_BRACE
LEFT_BRACKET
LEFT_PAREN
METRIC_IDENTIFIER
NUMBER
RIGHT_BRACE
RIGHT_BRACKET
RIGHT_PAREN
SEMICOLON
SPACE
STRING
TIMES

// Operators.
%token OPERATORS_START
ADD
DIV
EQLC
EQL_REGEX
GTE
GTR
LAND
LOR
LSS
LTE
LUNLESS
MOD
MUL
NEQ
NEQ_REGEX
POW
SUB
AT
ATAN2
%token OPERATORS_END

// Aggregators.
%token AGGREGATORS_START
AVG
BOTTOMK
COUNT
COUNT_VALUES
GROUP
MAX
MIN
QUANTILE
STDDEV
STDVAR
SUM
TOPK
%token AGGREGATORS_END

// Keywords.
%token KEYWORDS_START
BOOL
BY
GROUP_LEFT
GROUP_RIGHT
IGNORING
OFFSET
ON
WITHOUT
%token KEYWORDS_END

// Preprocessors.
%token PREPROCESSOR_START
START
END
%token PREPROCESSOR_END

// Start symbols for the generated parser.
%token STARTSYMBOLS_START
START_METRIC
START_SERIES_DESCRIPTION
START_EXPRESSION
START_METRIC_SELECTOR
%token STARTSYMBOLS_END

%start start

// Operators are listed with increasing precedence.
%left LOR
%left LAND LUNLESS
%left EQLC GTE GTR LSS LTE NEQ
%left ADD SUB
%left MUL DIV MOD ATAN2
%right POW

// Offset modifiers do not have associativity.
%nonassoc OFFSET

// This ensures that it is always attempted to parse range or subquery selectors when a left
// bracket is encountered.
%right LEFT_BRACKET

%%

start -> Result<Expr, String>:
                expr { $1 }
                /* If none of the more detailed error messages are triggered, we fall back to this. */
                | error { Err($1) }
                ;

expr -> Result<Expr, String>:
                aggregate_expr { $1 }
                /* binary_expr { $1 } */
                | function_call { $1 }
                | matrix_selector { $1 }
                | number_literal { $1 }
                | offset_expr { $1 }
                | paren_expr { $1 }
                | string_literal { $1 }
                | subquery_expr { $1 }
                /* | unary_expr  { $1 } */
                | vector_selector  { $1 }
                | step_invariant_expr { $1 }
                ;

/*
 * Aggregations.
 */
aggregate_expr -> Result<Expr, String>:
                aggregate_op aggregate_modifier function_call_body { Expr::new_aggregate_expr($1.id, $2?, $3?) }
                | aggregate_op function_call_body aggregate_modifier { Expr::new_aggregate_expr($1.id, $3?, $2?) }
                | aggregate_op function_call_body
                {
                        let modifier = AggModifier::By(HashSet::new());
                        Expr::new_aggregate_expr($1.id, modifier, $2?)
                }
                | aggregate_op error { Err($2) }
                ;

aggregate_modifier -> Result<AggModifier, String>:
                BY grouping_labels { Ok(AggModifier::By($2?)) }
                | WITHOUT grouping_labels { Ok(AggModifier::Without($2?)) }
                ;

/*
 * Binary expressions.
 */
// Operator precedence only works if each of those is listed separately.
binary_expr -> Result<Expr, String>:
                expr ADD       bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr ATAN2   bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr DIV     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr EQLC    bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr GTE     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr GTR     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr LAND    bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr LOR     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr LSS     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr LTE     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr LUNLESS bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr MOD     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr MUL     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr NEQ     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr POW     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                | expr SUB     bin_modifier expr { Expr::new_binary_expr($1?, $2.unwrap().tok_id(), $3?, $4?) }
                ;

// Using left recursion for the modifier rules, helps to keep the parser stack small and
// reduces allocations
bin_modifier -> Result<BinModifier, String>:
                group_modifiers { $1 }
                ;

bool_modifier -> Result<BinModifier, String>:
                {
                        let card = VectorMatchCardinality::OneToOne;
                        let matching =  VectorMatchModifier::On(HashSet::new());
                        let return_bool = false;
                        Ok(BinModifier {card, matching, return_bool})
                }
                | BOOL
                {
                        let card = VectorMatchCardinality::OneToOne;
                        let matching =  VectorMatchModifier::On(HashSet::new());
                        let return_bool = true;
                        Ok(BinModifier {card, matching, return_bool})
                }
                ;

on_or_ignoring -> Result<BinModifier, String>:
                bool_modifier IGNORING grouping_labels
                {
                        let mut modifier = $1?;
                        modifier.matching = VectorMatchModifier::Ignoring($3?);
                        Ok(modifier)
                }
                | bool_modifier ON grouping_labels
                {
                        let mut modifier = $1?;
                        modifier.matching = VectorMatchModifier::On($3?);
                        Ok(modifier)
                }
                ;

group_modifiers -> Result<BinModifier, String>:
                bool_modifier { $1 }
                | on_or_ignoring { $1 }
                | on_or_ignoring GROUP_LEFT maybe_grouping_labels
                {
                        let mut modifier = $1?;
                        modifier.card = VectorMatchCardinality::ManyToOne($3?);
                        Ok(modifier)
                }
                | on_or_ignoring GROUP_RIGHT maybe_grouping_labels
                {
                        let mut modifier = $1?;
                        modifier.card = VectorMatchCardinality::OneToMany($3?);
                        Ok(modifier)
                }
                ;

grouping_labels -> Result<Labels, String>:
                LEFT_PAREN grouping_label_list RIGHT_PAREN { $2 }
                | LEFT_PAREN grouping_label_list COMMA RIGHT_PAREN { $2 }
                | LEFT_PAREN RIGHT_PAREN { Ok(HashSet::new()) }
                | error { Err(format!("err in grouping opts {}", $1)) }
                ;

grouping_label_list -> Result<Labels, String>:
                grouping_label_list COMMA grouping_label
                {
                        let mut v = $1?;
                        v.insert($3?.val);
                        Ok(v)
                }
                | grouping_label { Ok(HashSet::from([$1?.val])) }
                | grouping_label_list error { Err(format!("err in grouping opts {}", $2)) }
                ;

grouping_label -> Result<Token, String>:
                maybe_label
                {
                        if is_label(&$1.val) {
                            Ok($1)
                        } else {
                            Err(format!("{} is not valid label in grouping opts", $1.val))
                        }
                }
                | error { Err($1) }
                ;

/*
 * Function calls.
 */
function_call -> Result<Expr, String>:
                IDENTIFIER function_call_body
                {
                        let name = lexeme_to_string($lexer, &$1)?;
                        match get_function(&name) {
                            None => Err(format!("unknown function with name {}", name)),
                            Some(func) => Expr::new_call(func, $2?)
                        }
                }
                ;

function_call_body -> Result<FunctionArgs, String>:
                LEFT_PAREN function_call_args RIGHT_PAREN { $2 }
                | LEFT_PAREN RIGHT_PAREN { Ok(FunctionArgs::empty_args()) }
                ;

function_call_args -> Result<FunctionArgs, String>:
                function_call_args COMMA expr { Ok($1?.append_args($3?)) }
                | expr { Ok(FunctionArgs::new_args($1?)) }
                | function_call_args COMMA { Err("trailing commas not allowed in function call args".into()) }
                ;

/*
 * Expressions inside parentheses.
 */
paren_expr -> Result<Expr, String>:
                LEFT_PAREN expr RIGHT_PAREN { Expr::new_paren_expr($2?) }
                ;

/*
 * Offset modifiers.
 */
offset_expr -> Result<Expr, String>:
                expr OFFSET duration { $1?.offset_expr(Offset::Pos($3?)) }
                | expr OFFSET SUB duration { $1?.offset_expr(Offset::Neg($4?)) }
                | expr OFFSET error { Err($3) }
                ;

/*
 * @ modifiers.
 */
step_invariant_expr -> Result<Expr, String>:
                expr AT signed_or_unsigned_number
                {
                        let at = AtModifier::try_from($3?)?;
                        $1?.step_invariant_expr(at)
                }
                | expr AT at_modifier_preprocessors LEFT_PAREN RIGHT_PAREN
                {
                        let at = AtModifier::try_from($3)?;
                        $1?.step_invariant_expr(at)
                }
                | expr AT error { Err($3) }
                ;

at_modifier_preprocessors -> Token:
                START { lexeme_to_token($lexer, $1) }
                | END { lexeme_to_token($lexer, $1) }
                ;

/*
 * Subquery and range selectors.
 */
matrix_selector -> Result<Expr, String>:
                expr LEFT_BRACKET duration RIGHT_BRACKET
                { Expr::new_matrix_selector($1?, $3?) }
                ;

subquery_expr -> Result<Expr, String>:
                expr LEFT_BRACKET duration COLON maybe_duration RIGHT_BRACKET
                { Expr::new_subquery_expr($1?, $3?, $5?) }
                | expr LEFT_BRACKET duration COLON duration error { Err($6) }
                | expr LEFT_BRACKET duration COLON error { Err($5) }
                | expr LEFT_BRACKET duration error { Err($4) }
                | expr LEFT_BRACKET error { Err($3) }
                ;

/*
 * Unary expressions.
 */
unary_expr -> Result<Expr, String>:
                /* gives the rule the same precedence as MUL. This aligns with mathematical conventions */
                /* FIXME: unary_op has same precedence with MUL, otherwise Rule Conflict */
                unary_op expr { Expr::new_unary_expr($2?, &$1) }
                ;

/*
 * Vector selectors.
 */
vector_selector -> Result<Expr, String>:
                metric_identifier label_matchers
                {
                        let name = $1.val;
                        let matcher = Matcher::new(MatchOp::Equal, METRIC_NAME.into(), name.clone());
                        let matchers = $2?.append(matcher);
                        Expr::new_vector_selector(Some(name), matchers)
                }
                | metric_identifier
                {
                        let name = $1.val;
                        let matcher = Matcher::new(MatchOp::Equal, METRIC_NAME.into(), name.clone());
                        let matchers = Matchers::empty().append(matcher);
                        Expr::new_vector_selector(Some(name), matchers)
                }
                | label_matchers { Expr::new_vector_selector(None, $1?) }
                ;

label_matchers -> Result<Matchers, String>:
                LEFT_BRACE label_match_list RIGHT_BRACE { $2 }
                | LEFT_BRACE label_match_list COMMA RIGHT_BRACE { $2 }
                | LEFT_BRACE RIGHT_BRACE { Ok(Matchers::empty()) }
                ;

label_match_list -> Result<Matchers, String>:
                label_match_list COMMA label_matcher { Ok($1?.append($3?)) }
                | label_matcher { Ok(Matchers::empty().append($1?)) }
                | label_match_list error { Err($2) }
                ;

label_matcher -> Result<Matcher, String>:
                IDENTIFIER match_op STRING
                {
                        let name = lexeme_to_string($lexer, &$1)?;
                        let value = lexeme_to_string($lexer, &$3)?;
                        Matcher::new_matcher($2.id, name, value)
                }
                | IDENTIFIER match_op error
                {
                        let id = lexeme_to_string($lexer, &$1)?;
                        let op = $2.val;
                        let err = $3;
                        Err(format!("matcher err. identifier:{id}, op:{op}, err:{err}"))
                }
                | IDENTIFIER error
                {
                        let id = lexeme_to_string($lexer, &$1)?;
                        let err = $2;
                        Err(format!("matcher err. identifier:{id}, err:{err}"))
                }
                | error
                {
                        let err = $1;
                        Err(format!("matcher err:{err}"))
                }
                ;

/*
 * Metric descriptions.
 */
metric_identifier -> Token:
                AVG { lexeme_to_token($lexer, $1) }
                | BOTTOMK { lexeme_to_token($lexer, $1) }
                | BY { lexeme_to_token($lexer, $1) }
                | COUNT { lexeme_to_token($lexer, $1) }
                | COUNT_VALUES { lexeme_to_token($lexer, $1) }
                | GROUP { lexeme_to_token($lexer, $1) }
                | IDENTIFIER { lexeme_to_token($lexer, $1) }
                | LAND { lexeme_to_token($lexer, $1) }
                | LOR { lexeme_to_token($lexer, $1) }
                | LUNLESS { lexeme_to_token($lexer, $1) }
                | MAX { lexeme_to_token($lexer, $1) }
                | METRIC_IDENTIFIER { lexeme_to_token($lexer, $1) }
                | MIN { lexeme_to_token($lexer, $1) }
                | OFFSET { lexeme_to_token($lexer, $1) }
                | QUANTILE { lexeme_to_token($lexer, $1) }
                | STDDEV { lexeme_to_token($lexer, $1) }
                | STDVAR { lexeme_to_token($lexer, $1) }
                | SUM { lexeme_to_token($lexer, $1) }
                | TOPK { lexeme_to_token($lexer, $1) }
                | WITHOUT { lexeme_to_token($lexer, $1) }
                | START { lexeme_to_token($lexer, $1) }
                | END { lexeme_to_token($lexer, $1) }
                ;

error -> String:
                ERROR { span_to_string($lexer, $span) }
                ;

/*
 * Series descriptions (only used by unit tests).
 * Note: this is not supported yet.
 */

/*
 * Keyword lists.
 */
aggregate_op -> Token:
                AVG { lexeme_to_token($lexer, $1) }
                | BOTTOMK { lexeme_to_token($lexer, $1) }
                | COUNT { lexeme_to_token($lexer, $1) }
                | COUNT_VALUES { lexeme_to_token($lexer, $1) }
                | GROUP { lexeme_to_token($lexer, $1) }
                | MAX { lexeme_to_token($lexer, $1) }
                | MIN { lexeme_to_token($lexer, $1) }
                | QUANTILE { lexeme_to_token($lexer, $1) }
                | STDDEV { lexeme_to_token($lexer, $1) }
                | STDVAR { lexeme_to_token($lexer, $1) }
                | SUM { lexeme_to_token($lexer, $1) }
                | TOPK { lexeme_to_token($lexer, $1) }
                ;

// inside of grouping options label names can be recognized as keywords by the lexer.
// This is a list of keywords that could also be a label name.
maybe_label -> Token:
                AVG { lexeme_to_token($lexer, $1) }
                | BOOL { lexeme_to_token($lexer, $1) }
                | BOTTOMK { lexeme_to_token($lexer, $1) }
                | BY { lexeme_to_token($lexer, $1) }
                | COUNT { lexeme_to_token($lexer, $1) }
                | COUNT_VALUES { lexeme_to_token($lexer, $1) }
                | GROUP { lexeme_to_token($lexer, $1) }
                | GROUP_LEFT { lexeme_to_token($lexer, $1) }
                | GROUP_RIGHT { lexeme_to_token($lexer, $1) }
                | IDENTIFIER { lexeme_to_token($lexer, $1) }
                | IGNORING { lexeme_to_token($lexer, $1) }
                | LAND { lexeme_to_token($lexer, $1) }
                | LOR { lexeme_to_token($lexer, $1) }
                | LUNLESS { lexeme_to_token($lexer, $1) }
                | MAX { lexeme_to_token($lexer, $1) }
                | METRIC_IDENTIFIER { lexeme_to_token($lexer, $1) }
                | MIN { lexeme_to_token($lexer, $1) }
                | OFFSET { lexeme_to_token($lexer, $1) }
                | ON { lexeme_to_token($lexer, $1) }
                | QUANTILE { lexeme_to_token($lexer, $1) }
                | STDDEV { lexeme_to_token($lexer, $1) }
                | STDVAR { lexeme_to_token($lexer, $1) }
                | SUM { lexeme_to_token($lexer, $1) }
                | TOPK { lexeme_to_token($lexer, $1) }
                | START { lexeme_to_token($lexer, $1) }
                | END { lexeme_to_token($lexer, $1) }
                | ATAN2 { lexeme_to_token($lexer, $1) }
                ;

unary_op -> Token:
                ADD { lexeme_to_token($lexer, $1) }
                | SUB { lexeme_to_token($lexer, $1) }
                ;

match_op -> Token:
                EQL { lexeme_to_token($lexer, $1) }
                | NEQ { lexeme_to_token($lexer, $1) }
                | EQL_REGEX { lexeme_to_token($lexer, $1) }
                | NEQ_REGEX { lexeme_to_token($lexer, $1) }
                ;

/*
 * Literals.
 */
number_literal -> Result<Expr, String>:
                signed_or_unsigned_number { Ok(Expr::from($1?)) }
                ;


signed_or_unsigned_number -> Result<f64, String>:
                number { $1 }
                | signed_number  { $1 }
                ;

signed_number -> Result<f64, String>:
                ADD number { $2 }
                | SUB number { $2.map(|i| -i) }
                ;

number -> Result<f64, String>:
                NUMBER { parse_golang_str_radix($lexer.span_str($span)) }
                ;

duration -> Result<Duration, String>:
                DURATION { parse_duration($lexer.span_str($span)) }
                ;

string_literal -> Result<Expr, String>:
                STRING { Ok(Expr::from(span_to_string($lexer, $span))) }
                ;

/*
 * Wrappers for optional arguments.
 */
maybe_duration -> Result<Duration, String>:
                { Ok(Duration::ZERO) }
                | duration { $1 }
                ;

maybe_grouping_labels -> Result<Labels, String>:
                { Ok(HashSet::new()) }
                | grouping_labels { $1 }
                ;

%%

use std::collections::HashSet;
use std::time::Duration;
use crate::label::{Labels, MatchOp, Matcher, Matchers, METRIC_NAME};
use crate::parser::{
    AggModifier, AtModifier, BinModifier, Expr, FunctionArgs, Offset, Token,
    VectorMatchCardinality, VectorMatchModifier,
    get_function, is_label, lexeme_to_string, lexeme_to_token, span_to_string,
};
use crate::util::{parse_duration, parse_golang_str_radix};
