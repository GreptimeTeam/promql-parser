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

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt::{self, Display};

lrlex::lrlex_mod!("token_map");
pub use token_map::*;

pub type TokenType = u8;

lazy_static! {
    static ref TOKEN_DISPLAY: HashMap<TokenType, &'static str> =
        [
            // Token.
            (T_EQL, "="),
            (T_BLANK, "_"),
            (T_COLON, ":"),
            (T_COMMA, ","),
            (T_COMMENT, "#"),
            (T_DURATION, "[du]"),
            (T_EOF, "<eof>"),
            (T_ERROR, "{Err}"),
            (T_IDENTIFIER, "{ID}"),
            (T_LEFT_BRACE, "{"),
            (T_LEFT_BRACKET, "["),
            (T_LEFT_PAREN, "("),
            (T_METRIC_IDENTIFIER, "{Metric_ID}"),
            (T_NUMBER, "{Num}"),
            (T_RIGHT_BRACE, "}"),
            (T_RIGHT_BRACKET, "]"),
            (T_RIGHT_PAREN, ")"),
            (T_SEMICOLON, ","),
            (T_SPACE, "<space>"),
            (T_STRING, "{Str}"),
            (T_TIMES, "x"),

            // Operators.
            (T_ADD, "+"),
            (T_DIV, "/"),
            (T_EQLC, "=="),
            (T_EQL_REGEX, "=~"),
            (T_GTE, ">="),
            (T_GTR, ">"),
            (T_LAND, "and"),
            (T_LOR, "or"),
            (T_LSS, "<"),
            (T_LTE, "<="),
            (T_LUNLESS, "unless"),
            (T_MOD, "%"),
            (T_MUL, "*"),
            (T_NEQ, "!="),
            (T_NEQ_REGEX, "!~"),
            (T_POW, "^"),
            (T_SUB, "-"),
            (T_AT, "@"),
            (T_ATAN2, "atan2"),

            // Aggregators.
            (T_AVG, "avg"),
            (T_BOTTOMK, "bottomk"),
            (T_COUNT, "count"),
            (T_COUNT_VALUES, "count_values"),
            (T_GROUP, "group"),
            (T_MAX, "max"),
            (T_MIN, "min"),
            (T_QUANTILE, "quantile"),
            (T_STDDEV, "stddev"),
            (T_STDVAR, "stdvar"),
            (T_SUM, "sum"),
            (T_TOPK, "topk"),

            // Keywords.
            (T_BOOL, "bool"),
            (T_BY, "by"),
            (T_GROUP_LEFT, "group_left"),
            (T_GROUP_RIGHT, "group_right"),
            (T_IGNORING, "ignoring"),
            (T_OFFSET, "offset"),
            (T_ON, "on"),
            (T_WITHOUT, "without"),

            // Preprocessors.
            (T_START, "start"),
            (T_END, "end")
        ].into_iter().collect();
}

/// this is for debug so far, maybe pretty feature in the future.
pub fn token_display(id: TokenType) -> &'static str {
    // match TOKEN_DISPLAY.get(&id) {
    //     Some(&display) => display.into(),
    //     None => format!("unknown token id <{id}>"),
    // }

    match id {
        // Token.
        T_EQL => "=",
        T_BLANK => "_",
        T_COLON => ":",
        T_COMMA => ",",
        T_COMMENT => "#",
        T_DURATION => "[du]",
        T_EOF => "<eof>",
        T_ERROR => "{Err}",
        T_IDENTIFIER => "{ID}",
        T_LEFT_BRACE => "{",
        T_LEFT_BRACKET => "[",
        T_LEFT_PAREN => "(",
        T_METRIC_IDENTIFIER => "{Metric_ID}",
        T_NUMBER => "{Num}",
        T_RIGHT_BRACE => "}",
        T_RIGHT_BRACKET => "]",
        T_RIGHT_PAREN => ")",
        T_SEMICOLON => ",",
        T_SPACE => "<space>",
        T_STRING => "{Str}",
        T_TIMES => "x",

        // Operators.
        T_ADD => "+",
        T_DIV => "/",
        T_EQLC => "==",
        T_EQL_REGEX => "=~",
        T_GTE => ">=",
        T_GTR => ">",
        T_LAND => "and",
        T_LOR => "or",
        T_LSS => "<",
        T_LTE => "<=",
        T_LUNLESS => "unless",
        T_MOD => "%",
        T_MUL => "*",
        T_NEQ => "!=",
        T_NEQ_REGEX => "!~",
        T_POW => "^",
        T_SUB => "-",
        T_AT => "@",
        T_ATAN2 => "atan2",

        // Aggregators.
        T_AVG => "avg",
        T_BOTTOMK => "bottomk",
        T_COUNT => "count",
        T_COUNT_VALUES => "count_values",
        T_GROUP => "group",
        T_MAX => "max",
        T_MIN => "min",
        T_QUANTILE => "quantile",
        T_STDDEV => "stddev",
        T_STDVAR => "stdvar",
        T_SUM => "sum",
        T_TOPK => "topk",

        // Keywords.
        T_BOOL => "bool",
        T_BY => "by",
        T_GROUP_LEFT => "group_left",
        T_GROUP_RIGHT => "group_right",
        T_IGNORING => "ignoring",
        T_OFFSET => "offset",
        T_ON => "on",
        T_WITHOUT => "without",

        // Preprocessors.
        T_START => "start",
        T_END => "end",

        _ => "unknown token",
    }
}

/// This is a list of all keywords in PromQL.
/// When changing this list, make sure to also change
/// the maybe_label grammar rule in the generated parser
/// to avoid misinterpretation of labels as keywords.
pub fn get_keyword_token(s: &str) -> Option<TokenType> {
    match s {
        // Operators.
        "and" => Some(T_LAND),
        "or" => Some(T_LOR),
        "unless" => Some(T_LUNLESS),
        "atan2" => Some(T_ATAN2),

        // Aggregators.
        "sum" => Some(T_SUM),
        "avg" => Some(T_AVG),
        "count" => Some(T_COUNT),
        "min" => Some(T_MIN),
        "max" => Some(T_MAX),
        "group" => Some(T_GROUP),
        "stddev" => Some(T_STDDEV),
        "stdvar" => Some(T_STDVAR),
        "topk" => Some(T_TOPK),
        "bottomk" => Some(T_BOTTOMK),
        "count_values" => Some(T_COUNT_VALUES),
        "quantile" => Some(T_QUANTILE),

        // Keywords.
        "offset" => Some(T_OFFSET),
        "by" => Some(T_BY),
        "without" => Some(T_WITHOUT),
        "on" => Some(T_ON),
        "ignoring" => Some(T_IGNORING),
        "group_left" => Some(T_GROUP_LEFT),
        "group_right" => Some(T_GROUP_RIGHT),
        "bool" => Some(T_BOOL),

        // Preprocessors.
        "start" => Some(T_START),
        "end" => Some(T_END),

        // Special numbers.
        "inf" => Some(T_NUMBER),
        "nan" => Some(T_NUMBER),

        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    id: TokenType,
    val: String,
}

impl Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "lexer token. id: {}, val: {}", self.id, self.val)
    }
}

impl Token {
    pub fn new(id: TokenType, val: String) -> Self {
        Self { id, val }
    }

    pub fn id(&self) -> TokenType {
        self.id
    }

    pub fn val(&self) -> String {
        self.val.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_display() {
        assert_eq!("@", token_display(T_AT));
        assert_eq!("unknown token", token_display(255));
    }

    #[test]
    fn test_get_keyword_tokens() {
        assert!(matches!(get_keyword_token("quantile"), Some(T_QUANTILE)));
        assert!(matches!(get_keyword_token("unknown"), None));
    }
}
