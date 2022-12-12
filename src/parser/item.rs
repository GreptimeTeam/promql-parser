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

use std::fmt::{self, Display};

type Pos = i32;

// Item represents a token or text string returned from the scanner.
#[derive(Debug)]
pub struct Item {
    typ: ItemType, // The type of this Item.
    pos: Pos,      // The starting position, in bytes, of this Item in the input string.
    val: String,   // The value of this Item.
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "item {}", self.val)
    }
}

#[derive(Debug)]
pub enum ItemType {
    TokenItemType,
    OperatorItemType,
    AggregatorItemType,
    KeywordItemType,
    PreprocessorsItemType,
}

#[derive(Debug)]
pub enum TokenItemType {
    Eql,
    Blank,
    Colon,
    Comma,
    Comment,
    Duration,
    Eof,
    Error,
    Identifier,
    LeftBrace,
    LeftBracket,
    LeftParen,
    MetricIdentifier,
    Number,
    RightBrace,
    RightBracket,
    RightParen,
    Semicolon,
    Space,
    String,
    Times,
}

#[derive(Debug)]
pub enum OperatorItemType {
    Add,
    Div,
    Eqlc,
    EqlRegex,
    Gte,
    Gtr,
    Land,
    Lor,
    Lss,
    Lte,
    Lunless,
    Mod,
    Mul,
    Neq,
    NeqRegex,
    Pow,
    Sub,
    At,
    Atan2,
}

#[derive(Debug)]
pub enum AggregatorItemType {
    Avg,
    Bottomk,
    Count,
    CountValues,
    Group,
    Max,
    Min,
    Quantile,
    Stddev,
    Stdvar,
    Sum,
    Topk,
}

#[derive(Debug)]
pub enum KeywordItemType {
    Bool,
    By,
    GroupLeft,
    GroupRight,
    Ignoring,
    Offset,
    On,
    Without,
}

#[derive(Debug)]
pub enum PreprocessorsItemType {
    Start,
    End,
}
