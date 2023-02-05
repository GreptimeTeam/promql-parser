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

%start expr

// Operators are listed with increasing precedence.
%left LOR
%left LAND LUNLESS
%left EQLC GTE GTR LSS LTE NEQ
%left ADD SUB
%left MUL DIV MOD ATAN2
%right POW

// Offset and At modifiers do not have associativity.
%nonassoc OFFSET AT

// This ensures that it is always attempted to parse range or subquery selectors when a left
// bracket is encountered.
%right LEFT_BRACKET

%%
expr -> Result<Expr, String>:
                aggregate_expr { check_ast($1?) }
                | at_expr { check_ast($1?) }
                | binary_expr { check_ast($1?) }
                | function_call { check_ast($1?) }
                | matrix_selector { check_ast($1?) }
                | number_literal { check_ast($1?) }
                | offset_expr { check_ast($1?) }
                | paren_expr { check_ast($1?) }
                | string_literal { check_ast($1?) }
                | subquery_expr { check_ast($1?) }
                | unary_expr  { check_ast($1?) }
                | vector_selector  { check_ast($1?) }
                ;

/*
 * Aggregations.
 */
aggregate_expr -> Result<Expr, String>:
                aggregate_op aggregate_modifier function_call_body
                {
                        Expr::new_aggregate_expr($1?.id, $2?, $3?)
                }
                | aggregate_op function_call_body aggregate_modifier
                {
                        Expr::new_aggregate_expr($1?.id, $3?, $2?)
                }
                | aggregate_op function_call_body
                {
                        let modifier = AggModifier::By(HashSet::new());
                        Expr::new_aggregate_expr($1?.id, modifier, $2?)
                }
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
bin_modifier -> Result<Option<BinModifier>, String>:
                group_modifiers { $1 }
                ;

bool_modifier -> Result<Option<BinModifier>, String>:
                { Ok(None) }
                | BOOL
                {
                        let modifier = BinModifier::default_modifier().return_bool(true);
                        Ok(Some(modifier))
                }
                ;

on_or_ignoring -> Result<Option<BinModifier>, String>:
                bool_modifier IGNORING grouping_labels
                {
                        Ok(BinModifier::update_matching($1?, Some(VectorMatchModifier::Ignoring($3?))))
                }
                | bool_modifier ON grouping_labels
                {
                        Ok(BinModifier::update_matching($1?, Some(VectorMatchModifier::On($3?))))
                }
                ;

/* FIXME: group_op without labels */
group_modifiers -> Result<Option<BinModifier>, String>:
                bool_modifier { $1 }
                | on_or_ignoring { $1 }
                | on_or_ignoring GROUP_LEFT grouping_labels
                {
                        Ok(BinModifier::update_card($1?, VectorMatchCardinality::ManyToOne($3?)))
                }
                | on_or_ignoring GROUP_RIGHT grouping_labels
                {
                        Ok(BinModifier::update_card($1?, VectorMatchCardinality::OneToMany($3?)))
                }
                ;

grouping_labels -> Result<Labels, String>:
                LEFT_PAREN grouping_label_list RIGHT_PAREN { $2 }
                | LEFT_PAREN grouping_label_list COMMA RIGHT_PAREN { $2 }
                | LEFT_PAREN RIGHT_PAREN { Ok(HashSet::new()) }
                ;

grouping_label_list -> Result<Labels, String>:
                grouping_label_list COMMA grouping_label
                {
                        let mut v = $1?;
                        v.insert($3?.val);
                        Ok(v)
                }
                | grouping_label { Ok(HashSet::from([$1?.val])) }
                ;

grouping_label -> Result<Token, String>:
                maybe_label
                {
                        let token = $1?;
                        let label = &token.val;
                        if is_label(label) {
                            Ok(token)
                        } else {
                            Err(format!("{label} is not valid label in grouping opts"))
                        }
                }
                ;

/*
 * Function calls.
 */
function_call -> Result<Expr, String>:
                IDENTIFIER function_call_body
                {
                        let name = lexeme_to_string($lexer, &$1)?;
                        match get_function(&name) {
                            None => Err(format!("unknown function with name {name}")),
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
                | expr OFFSET ADD duration { $1?.offset_expr(Offset::Pos($4?)) }
                | expr OFFSET SUB duration { $1?.offset_expr(Offset::Neg($4?)) }
                ;

/*
 * @ modifiers.
 *
 * the original name of this production head is step_invariant_expr
 */
at_expr -> Result<Expr, String>:
                expr AT number_literal { $1?.at_expr(AtModifier::try_from($3?)?) }
                | expr AT ADD number_literal { $1?.at_expr(AtModifier::try_from($4?)?) }
                | expr AT SUB number_literal
                {
                        let nl = $4.map(|nl| -nl);
                        $1?.at_expr(AtModifier::try_from(nl?)?)
                }
                | expr AT at_modifier_preprocessors LEFT_PAREN RIGHT_PAREN
                {
                        let at = AtModifier::try_from($3?)?;
                        $1?.at_expr(at)
                }
                ;

at_modifier_preprocessors -> Result<Token, String>:
                START { lexeme_to_token($lexer, $1) }
                | END { lexeme_to_token($lexer, $1) }
                ;

/*
 * Subquery and range selectors.
 */
matrix_selector -> Result<Expr, String>:
                expr LEFT_BRACKET duration RIGHT_BRACKET
                {
                        Expr::new_matrix_selector($1?, $3?)
                }
                ;

subquery_expr -> Result<Expr, String>:
                expr LEFT_BRACKET duration COLON maybe_duration RIGHT_BRACKET
                {
                        Expr::new_subquery_expr($1?, $3?, $5?)
                }
                ;

/*
 * Unary expressions.
 */
unary_expr -> Result<Expr, String>:
                ADD expr %prec MUL { $2 }
                | SUB expr %prec MUL { Expr::new_unary_expr($2?) }
                ;

/*
 * Vector selectors.
 */
vector_selector -> Result<Expr, String>:
                metric_identifier label_matchers
                {
                        let name = $1?.val;
                        let matcher = Matcher::new_eq_metric_matcher(name.clone());
                        let matchers = $2?.append(matcher);
                        Expr::new_vector_selector(Some(name), matchers)
                }
                | metric_identifier
                {
                        let name = $1?.val;
                        let matcher = Matcher::new_eq_metric_matcher(name.clone());
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
                ;

label_matcher -> Result<Matcher, String>:
                IDENTIFIER match_op STRING
                {
                        let name = lexeme_to_string($lexer, &$1)?;
                        let value = lexeme_to_string($lexer, &$3)?;
                        Matcher::new_matcher($2?.id, name, value)
                }
                ;

/*
 * Metric descriptions.
 */
metric_identifier -> Result<Token, String>:
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

/*
 * Series descriptions (only used by unit tests).
 * Note: this is not supported yet.
 */

/*
 * Keyword lists.
 */
aggregate_op -> Result<Token, String>:
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
maybe_label -> Result<Token, String>:
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

match_op -> Result<Token, String>:
                EQL { lexeme_to_token($lexer, $1) }
                | NEQ { lexeme_to_token($lexer, $1) }
                | EQL_REGEX { lexeme_to_token($lexer, $1) }
                | NEQ_REGEX { lexeme_to_token($lexer, $1) }
                ;

/*
 * Literals.
 */
number_literal -> Result<Expr, String>:
                NUMBER
                {
                        let num = parse_str_radix($lexer.span_str($span));
                        Ok(Expr::from(num?))
                }
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

maybe_duration -> Result<Option<Duration>, String>:
                { Ok(None) }
                | duration
                {
                        $1.map(Some)
                }
                ;

%%

use std::collections::HashSet;
use std::time::Duration;
use crate::label::{Labels, Matcher, Matchers};
use crate::parser::{
    AggModifier, AtModifier, BinModifier, Expr, FunctionArgs,
    Offset, Token, VectorMatchCardinality, VectorMatchModifier,
    check_ast, get_function, is_label,
    lexeme_to_string, lexeme_to_token, span_to_string,
};
use crate::util::{parse_duration, parse_str_radix};
