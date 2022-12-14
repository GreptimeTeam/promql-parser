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

// fn is_operator(id: TokenType) -> bool {
//     id > T_OPERATORS_START && id < T_OPERATORS_END
// }

// fn is_aggregator(id: TokenType) -> bool {
//     id > T_AGGREGATORS_START && id < T_AGGREGATORS_END
// }

// fn is_aggregator_with_param(id: TokenType) -> bool {
//     id == T_TOPK || id == T_BOTTOMK || id == T_COUNT_VALUES || id == T_QUANTILE
// }

// fn is_keyword(id: TokenType) -> bool {
//     id > T_KEYWORDS_START && id < T_KEYWORDS_END
// }

// // IsComparisonOperator returns true if the Item corresponds to a comparison operator.
// // Returns false otherwise.
// fn iscomparisonoperator(id: TokenType) -> bool {
//     id == T_EQLC || id == T_NEQ || id == T_LTE || id == T_LSS || id == T_GTE || id == T_GTR
// }

// fn is_set_operator(id: TokenType) -> bool {
//     id == T_LAND || id == T_LOR || id == T_LUNLESS
// }

lazy_static! {
    static ref TOKEN_DISPLAY: HashMap<TokenType, &'static str> = {
        let mut m = HashMap::new();

        // Operators.
        m.insert(T_LAND, "and");
        m.insert(T_LOR, "or");
        m.insert(T_LUNLESS, "unless");
        m.insert(T_ATAN2, "atan2");

        // Aggregators.
        m.insert(T_SUM, "sum");
        m.insert(T_AVG, "avg");
        m.insert(T_COUNT, "count");
        m.insert(T_MIN, "min");
        m.insert(T_MAX, "max");
        m.insert(T_GROUP, "group");
        m.insert(T_STDDEV, "stddev");
        m.insert(T_STDVAR, "stdvar");
        m.insert(T_TOPK, "topk");
        m.insert(T_BOTTOMK, "bottomk");
        m.insert(T_COUNT_VALUES, "count_values");
        m.insert(T_QUANTILE, "quantile");

        // Keywords.
        m.insert(T_OFFSET, "offset");
        m.insert(T_BY, "by");
        m.insert(T_WITHOUT, "without");
        m.insert(T_ON, "on");
        m.insert(T_IGNORING, "ignoring");
        m.insert(T_GROUP_LEFT, "group_left");
        m.insert(T_GROUP_RIGHT, "group_right");
        m.insert(T_BOOL, "bool");

        // Preprocessors.
        m.insert(T_START, "start");
        m.insert(T_END, "end");

        m.insert(T_LEFT_PAREN, "(");
        m.insert(T_RIGHT_PAREN, ")");
        m.insert(T_LEFT_BRACE, "{");
        m.insert(T_RIGHT_BRACE, "}");
        m.insert(T_LEFT_BRACKET, "[");
        m.insert(T_RIGHT_BRACKET, "]");
        m.insert(T_COMMA, ",");
        m.insert(T_EQL, "=");
        m.insert(T_COLON, ":");
        m.insert(T_SEMICOLON, ";");
        m.insert(T_BLANK, "_");
        m.insert(T_TIMES, "x");
        m.insert(T_SPACE, "<space>");
        m.insert(T_SUB, "-");
        m.insert(T_ADD, "+");
        m.insert(T_MUL, "*");
        m.insert(T_MOD, "%");
        m.insert(T_DIV, "/");
        m.insert(T_EQLC, "==");
        m.insert(T_NEQ, "!=");
        m.insert(T_LTE, "<=");
        m.insert(T_LSS, "<");
        m.insert(T_GTE, ">=");
        m.insert(T_GTR, ">");
        m.insert(T_EQL_REGEX, "=~");
        m.insert(T_NEQ_REGEX, "!~");
        m.insert(T_POW, "^");

        m
    };
}

pub fn token_display(id: TokenType) -> String {
    match TOKEN_DISPLAY.get(&id) {
        Some(&display) => display.into(),
        None => format!("unknown token id <{id}>"),
    }
}

#[derive(Debug)]
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
