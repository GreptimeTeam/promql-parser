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
                vector_selector { $1 }
                | string_literal { $1 }
                | number_literal { $1 }
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
                        let name = lexeme_to_string($lexer, &$1);
                        let value = lexeme_to_string($lexer, &$3);
                        Matcher::new_matcher($2.id, name, value)
                }
                | IDENTIFIER match_op error
                {
                        let id = lexeme_to_string($lexer, &$1);
                        let op = $2.val;
                        let err = $3;
                        Err(format!("matcher err. identifier:{id}, op:{op}, err:{err}"))
                }
                | IDENTIFIER error
                {
                        let id = lexeme_to_string($lexer, &$1);
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
metric -> Result<Labels, String>:
                metric_identifier label_set
                {
                        let label = Label::new(METRIC_NAME.to_string(), $1.val);
                        Ok($2?.append(label))
                }
                | label_set { $1 }
                ;


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

label_set -> Result<Labels, String>:
                LEFT_BRACE label_set_list RIGHT_BRACE { $2 }
                | LEFT_BRACE label_set_list COMMA RIGHT_BRACE { $2 }
                | LEFT_BRACE RIGHT_BRACE { Ok(Labels::empty()) }
                ;

label_set_list -> Result<Labels, String>:
                label_set_list COMMA label_set_item { Ok($1?.append($3?)) }
                | label_set_item { Ok(Labels::new(vec![$1?])) }
                ;

label_set_item -> Result<Label, String>:
                IDENTIFIER EQL STRING
                {
                        let name = lexeme_to_string($lexer, &$1);
                        let value = lexeme_to_string($lexer, &$3);
                        Ok(Label::new(name, value))
                }
                | IDENTIFIER EQL error
                {
                        let err = $3;
                        Err(format!("label set error, {err}"))
                }
                | IDENTIFIER error
                {
                        let err = $2;
                        Err(format!("label set error, {err}"))
                }
                | error
                {
                        let err = $1;
                        Err(format!("label set error, {err}"))
                }
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
                signed_or_unsigned_number { Expr::new_number_literal($1?) }
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
                NUMBER
                {
                        let s = $lexer.span_str($span);
                        parse_golang_str_radix(s)
                }
                ;

duration -> Result<Duration, String>:
                DURATION { parse_duration($lexer.span_str($span)) }
                ;

string_literal -> Result<Expr, String>:
                STRING { Expr::new_string_literal(span_to_string($lexer, $span)) }
                ;

/*
 * Wrappers for optional arguments.
 */
/* FIXME: rebase after grouping_labels rule is merged */
/* maybe_duration -> Result<Duration, String>: */
/*                 { Ok(Duration::ZERO) } */
/*                 | duration { $1 } */
/*                 ; */

/* maybe_grouping_labels -> Result<Vec<String>, String>: */
/*                 { Ok(vec![]) } */
/*                 | grouping_labels { $1 } */
/*                 ; */

%%
use std::time::Duration;

/* FIXME: rebase after rules are merged */
use crate::parser::{
    Expr, Token, lexeme_to_string, lexeme_to_token, span_to_string,
};
use crate::label::{Label, Labels, MatchOp, Matcher, Matchers, METRIC_NAME};
use crate::util::{parse_duration, parse_golang_str_radix};
