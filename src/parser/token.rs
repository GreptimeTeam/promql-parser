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

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt;

lrlex::lrlex_mod!("token_map");
pub use token_map::*;

pub type TokenId = u8;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TokenType(TokenId);

#[cfg(feature = "ser")]
impl serde::Serialize for TokenType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(token_display(self.0))
    }
}

lazy_static! {
    static ref KEYWORDS: HashMap<&'static str, TokenId> =
        [
            // Operators.
            ("and", T_LAND),
            ("or", T_LOR),
            ("unless", T_LUNLESS),
            ("atan2", T_ATAN2),

            // Aggregators.
            ("sum", T_SUM),
            ("avg", T_AVG),
            ("count", T_COUNT),
            ("min", T_MIN),
            ("max", T_MAX),
            ("group", T_GROUP),
            ("stddev", T_STDDEV),
            ("stdvar", T_STDVAR),
            ("topk", T_TOPK),
            ("bottomk", T_BOTTOMK),
            ("count_values", T_COUNT_VALUES),
            ("quantile", T_QUANTILE),
            ("limitk", T_LIMITK),
            ("limit_ratio", T_LIMIT_RATIO),

            // Keywords.
            ("offset", T_OFFSET),
            ("by", T_BY),
            ("without", T_WITHOUT),
            ("on", T_ON),
            ("ignoring", T_IGNORING),
            ("group_left", T_GROUP_LEFT),
            ("group_right", T_GROUP_RIGHT),
            ("bool", T_BOOL),
            ("smoothed", T_SMOOTHED),
            ("anchored", T_ANCHORED),

            // Preprocessors.
            ("start", T_START),
            ("end", T_END),

            // Special numbers.
            ("inf", T_NUMBER),
            ("nan", T_NUMBER),
        ].into_iter().collect();
}

/// this is for debug so far, maybe pretty feature in the future.
#[allow(dead_code)]
pub(crate) fn token_display(id: TokenId) -> &'static str {
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
        T_OPEN_HIST => "{{",
        T_CLOSE_HIST => "}}",
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
        T_OPERATORS_START => "operators_start",
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
        T_OPERATORS_END => "operators_end",

        // Aggregators.
        T_AGGREGATORS_START => "aggregators_start",
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
        T_LIMITK => "limitk",
        T_LIMIT_RATIO => "limit_ratio",
        T_AGGREGATORS_END => "aggregators_end",

        // Keywords.
        T_KEYWORDS_START => "keywords_start",
        T_BOOL => "bool",
        T_BY => "by",
        T_GROUP_LEFT => "group_left",
        T_GROUP_RIGHT => "group_right",
        T_IGNORING => "ignoring",
        T_OFFSET => "offset",
        T_SMOOTHED => "smoothed",
        T_ANCHORED => "anchored",
        T_ON => "on",
        T_WITHOUT => "without",
        T_KEYWORDS_END => "keywords_end",

        // Preprocessors.
        T_PREPROCESSOR_START => "preprocessor_start",
        T_START => "start",
        T_END => "end",
        T_STEP => "step",
        T_PREPROCESSOR_END => "preprocessors_end",

        T_STARTSYMBOLS_START
        | T_START_METRIC
        | T_START_SERIES_DESCRIPTION
        | T_START_EXPRESSION
        | T_START_METRIC_SELECTOR
        | T_STARTSYMBOLS_END => "not used",

        _ => "unknown token",
    }
}

/// This is a list of all keywords in PromQL.
/// When changing this list, make sure to also change
/// maybe_label grammar rule in generated parser
/// to avoid misinterpretation of labels as keywords.
pub(crate) fn get_keyword_token(s: &str) -> Option<TokenId> {
    KEYWORDS.get(s).copied()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub id: TokenType,
    pub val: String,
}

impl Token {
    pub fn new(id: TokenId, val: String) -> Self {
        Self {
            id: TokenType(id),
            val,
        }
    }

    pub fn id(&self) -> TokenId {
        self.id.id()
    }
}

impl TokenType {
    pub fn new(id: TokenId) -> Self {
        Self(id)
    }

    pub fn id(&self) -> TokenId {
        self.0
    }

    pub fn is_aggregator(&self) -> bool {
        self.0 > T_AGGREGATORS_START && self.0 < T_AGGREGATORS_END
    }

    pub fn is_aggregator_with_param(&self) -> bool {
        matches!(self.0, T_TOPK | T_BOTTOMK | T_COUNT_VALUES | T_QUANTILE)
    }

    pub fn is_comparison_operator(&self) -> bool {
        matches!(self.0, T_EQLC | T_NEQ | T_LTE | T_LSS | T_GTE | T_GTR)
    }

    pub fn is_set_operator(&self) -> bool {
        matches!(self.0, T_LAND | T_LOR | T_LUNLESS)
    }

    pub fn is_operator(&self) -> bool {
        self.0 > T_OPERATORS_START && self.0 < T_OPERATORS_END
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", token_display(self.id()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_display() {
        assert_eq!(token_display(T_EQL), "=");
        assert_eq!(token_display(T_BLANK), "_");
        assert_eq!(token_display(T_COLON), ":");
        assert_eq!(token_display(T_COMMA), ",");
        assert_eq!(token_display(T_COMMENT), "#");
        assert_eq!(token_display(T_DURATION), "[du]");
        assert_eq!(token_display(T_EOF), "<eof>");
        assert_eq!(token_display(T_ERROR), "{Err}");
        assert_eq!(token_display(T_IDENTIFIER), "{ID}");
        assert_eq!(token_display(T_LEFT_BRACE), "{");
        assert_eq!(token_display(T_LEFT_BRACKET), "[");
        assert_eq!(token_display(T_LEFT_PAREN), "(");
        assert_eq!(token_display(T_OPEN_HIST), "{{");
        assert_eq!(token_display(T_CLOSE_HIST), "}}");
        assert_eq!(token_display(T_METRIC_IDENTIFIER), "{Metric_ID}");
        assert_eq!(token_display(T_NUMBER), "{Num}");
        assert_eq!(token_display(T_RIGHT_BRACE), "}");
        assert_eq!(token_display(T_RIGHT_BRACKET), "]");
        assert_eq!(token_display(T_RIGHT_PAREN), ")");
        assert_eq!(token_display(T_SEMICOLON), ",");
        assert_eq!(token_display(T_SPACE), "<space>");
        assert_eq!(token_display(T_STRING), "{Str}");
        assert_eq!(token_display(T_TIMES), "x");
        assert_eq!(token_display(T_OPERATORS_START), "operators_start");
        assert_eq!(token_display(T_ADD), "+");
        assert_eq!(token_display(T_DIV), "/");
        assert_eq!(token_display(T_EQLC), "==");
        assert_eq!(token_display(T_EQL_REGEX), "=~");
        assert_eq!(token_display(T_GTE), ">=");
        assert_eq!(token_display(T_GTR), ">");
        assert_eq!(token_display(T_LAND), "and");
        assert_eq!(token_display(T_LOR), "or");
        assert_eq!(token_display(T_LSS), "<");
        assert_eq!(token_display(T_LTE), "<=");
        assert_eq!(token_display(T_LUNLESS), "unless");
        assert_eq!(token_display(T_MOD), "%");
        assert_eq!(token_display(T_MUL), "*");
        assert_eq!(token_display(T_NEQ), "!=");
        assert_eq!(token_display(T_NEQ_REGEX), "!~");
        assert_eq!(token_display(T_POW), "^");
        assert_eq!(token_display(T_SUB), "-");
        assert_eq!(token_display(T_AT), "@");
        assert_eq!(token_display(T_ATAN2), "atan2");
        assert_eq!(token_display(T_OPERATORS_END), "operators_end");
        assert_eq!(token_display(T_AGGREGATORS_START), "aggregators_start");
        assert_eq!(token_display(T_AVG), "avg");
        assert_eq!(token_display(T_BOTTOMK), "bottomk");
        assert_eq!(token_display(T_COUNT), "count");
        assert_eq!(token_display(T_COUNT_VALUES), "count_values");
        assert_eq!(token_display(T_GROUP), "group");
        assert_eq!(token_display(T_MAX), "max");
        assert_eq!(token_display(T_MIN), "min");
        assert_eq!(token_display(T_QUANTILE), "quantile");
        assert_eq!(token_display(T_STDDEV), "stddev");
        assert_eq!(token_display(T_STDVAR), "stdvar");
        assert_eq!(token_display(T_SUM), "sum");
        assert_eq!(token_display(T_TOPK), "topk");
        assert_eq!(token_display(T_LIMITK), "limitk");
        assert_eq!(token_display(T_LIMIT_RATIO), "limit_ratio");
        assert_eq!(token_display(T_AGGREGATORS_END), "aggregators_end");
        assert_eq!(token_display(T_KEYWORDS_START), "keywords_start");
        assert_eq!(token_display(T_BOOL), "bool");
        assert_eq!(token_display(T_BY), "by");
        assert_eq!(token_display(T_GROUP_LEFT), "group_left");
        assert_eq!(token_display(T_GROUP_RIGHT), "group_right");
        assert_eq!(token_display(T_IGNORING), "ignoring");
        assert_eq!(token_display(T_OFFSET), "offset");
        assert_eq!(token_display(T_SMOOTHED), "smoothed");
        assert_eq!(token_display(T_ANCHORED), "anchored");
        assert_eq!(token_display(T_ON), "on");
        assert_eq!(token_display(T_WITHOUT), "without");
        assert_eq!(token_display(T_KEYWORDS_END), "keywords_end");
        assert_eq!(token_display(T_PREPROCESSOR_START), "preprocessor_start");
        assert_eq!(token_display(T_START), "start");
        assert_eq!(token_display(T_END), "end");
        assert_eq!(token_display(T_STEP), "step");
        assert_eq!(token_display(T_PREPROCESSOR_END), "preprocessors_end");

        // if new token added in promql.y, this has to be updated
        for i in 77..=82 {
            assert_eq!(token_display(i), "not used");
        }

        // All tokens are now tested individually above

        for i in 83..=255 {
            assert_eq!(token_display(i), "unknown token");
        }
    }

    #[test]
    fn test_get_keyword_tokens() {
        assert!(matches!(get_keyword_token("and"), Some(T_LAND)));
        assert!(matches!(get_keyword_token("or"), Some(T_LOR)));
        assert!(matches!(get_keyword_token("unless"), Some(T_LUNLESS)));
        assert!(matches!(get_keyword_token("atan2"), Some(T_ATAN2)));
        assert!(matches!(get_keyword_token("sum"), Some(T_SUM)));
        assert!(matches!(get_keyword_token("avg"), Some(T_AVG)));
        assert!(matches!(get_keyword_token("count"), Some(T_COUNT)));
        assert!(matches!(get_keyword_token("min"), Some(T_MIN)));
        assert!(matches!(get_keyword_token("max"), Some(T_MAX)));
        assert!(matches!(get_keyword_token("group"), Some(T_GROUP)));
        assert!(matches!(get_keyword_token("stddev"), Some(T_STDDEV)));
        assert!(matches!(get_keyword_token("stdvar"), Some(T_STDVAR)));
        assert!(matches!(get_keyword_token("topk"), Some(T_TOPK)));
        assert!(matches!(get_keyword_token("bottomk"), Some(T_BOTTOMK)));
        assert!(matches!(
            get_keyword_token("count_values"),
            Some(T_COUNT_VALUES)
        ));
        assert!(matches!(get_keyword_token("quantile"), Some(T_QUANTILE)));
        assert!(matches!(get_keyword_token("offset"), Some(T_OFFSET)));
        assert!(matches!(get_keyword_token("by"), Some(T_BY)));
        assert!(matches!(get_keyword_token("without"), Some(T_WITHOUT)));
        assert!(matches!(get_keyword_token("on"), Some(T_ON)));
        assert!(matches!(get_keyword_token("ignoring"), Some(T_IGNORING)));
        assert!(matches!(
            get_keyword_token("group_left"),
            Some(T_GROUP_LEFT)
        ));
        assert!(matches!(
            get_keyword_token("group_right"),
            Some(T_GROUP_RIGHT)
        ));
        assert!(matches!(get_keyword_token("bool"), Some(T_BOOL)));
        assert!(matches!(get_keyword_token("start"), Some(T_START)));
        assert!(matches!(get_keyword_token("end"), Some(T_END)));
        assert!(matches!(get_keyword_token("inf"), Some(T_NUMBER)));
        assert!(matches!(get_keyword_token("nan"), Some(T_NUMBER)));

        // not keywords
        assert!(get_keyword_token("at").is_none());
        assert!(get_keyword_token("unknown").is_none());
    }

    #[test]
    fn test_with_param() {
        assert!(TokenType(T_TOPK).is_aggregator_with_param());
        assert!(TokenType(T_BOTTOMK).is_aggregator_with_param());
        assert!(TokenType(T_COUNT_VALUES).is_aggregator_with_param());
        assert!(TokenType(T_QUANTILE).is_aggregator_with_param());

        assert!(!TokenType(T_MAX).is_aggregator_with_param());
        assert!(!TokenType(T_MIN).is_aggregator_with_param());
        assert!(!TokenType(T_AVG).is_aggregator_with_param());
    }

    #[test]
    fn test_comparison_operator() {
        assert!(TokenType(T_EQLC).is_comparison_operator());
        assert!(TokenType(T_NEQ).is_comparison_operator());
        assert!(TokenType(T_LTE).is_comparison_operator());
        assert!(TokenType(T_LSS).is_comparison_operator());
        assert!(TokenType(T_GTE).is_comparison_operator());
        assert!(TokenType(T_GTR).is_comparison_operator());

        assert!(!TokenType(T_ADD).is_comparison_operator());
        assert!(!TokenType(T_LAND).is_comparison_operator());
    }

    #[test]
    fn test_is_set_operator() {
        assert!(TokenType(T_LAND).is_set_operator());
        assert!(TokenType(T_LOR).is_set_operator());
        assert!(TokenType(T_LUNLESS).is_set_operator());

        assert!(!TokenType(T_ADD).is_set_operator());
        assert!(!TokenType(T_MAX).is_set_operator());
        assert!(!TokenType(T_NEQ).is_set_operator());
    }

    #[test]
    fn test_is_operator() {
        assert!(TokenType(T_ADD).is_operator());
        assert!(TokenType(T_DIV).is_operator());
        assert!(TokenType(T_EQLC).is_operator());
        assert!(TokenType(T_EQL_REGEX).is_operator());
        assert!(TokenType(T_GTE).is_operator());
        assert!(TokenType(T_GTR).is_operator());
        assert!(TokenType(T_LAND).is_operator());
        assert!(TokenType(T_LOR).is_operator());
        assert!(TokenType(T_LSS).is_operator());
        assert!(TokenType(T_LTE).is_operator());
        assert!(TokenType(T_LUNLESS).is_operator());
        assert!(TokenType(T_MOD).is_operator());
        assert!(TokenType(T_MUL).is_operator());
        assert!(TokenType(T_NEQ).is_operator());
        assert!(TokenType(T_NEQ_REGEX).is_operator());
        assert!(TokenType(T_POW).is_operator());
        assert!(TokenType(T_SUB).is_operator());
        assert!(TokenType(T_AT).is_operator());
        assert!(TokenType(T_ATAN2).is_operator());

        assert!(!TokenType(T_SUM).is_operator());
        assert!(!TokenType(T_OPERATORS_START).is_operator());
        assert!(!TokenType(T_OPERATORS_END).is_operator());
    }

    #[test]
    fn test_is_aggregator() {
        assert!(TokenType(T_AVG).is_aggregator());
        assert!(TokenType(T_BOTTOMK).is_aggregator());
        assert!(TokenType(T_COUNT).is_aggregator());
        assert!(TokenType(T_COUNT_VALUES).is_aggregator());
        assert!(TokenType(T_GROUP).is_aggregator());
        assert!(TokenType(T_MAX).is_aggregator());
        assert!(TokenType(T_MIN).is_aggregator());
        assert!(TokenType(T_QUANTILE).is_aggregator());
        assert!(TokenType(T_STDDEV).is_aggregator());
        assert!(TokenType(T_STDVAR).is_aggregator());
        assert!(TokenType(T_SUM).is_aggregator());
        assert!(TokenType(T_TOPK).is_aggregator());

        assert!(!TokenType(T_LOR).is_aggregator());
        assert!(!TokenType(T_ADD).is_aggregator());
    }
}
