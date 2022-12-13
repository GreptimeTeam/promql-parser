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

use crate::parser::{LexemeType, Token, TokenType};
use lrpar::{Lexeme, NonStreamingLexer, Span};

/// caller MUST pay attention to the index out of bounds issue
pub fn span_to_string(lexer: &dyn NonStreamingLexer<LexemeType, TokenType>, span: Span) -> String {
    lexer.span_str(span).to_string()
}

pub fn lexeme_to_string(
    lexer: &dyn NonStreamingLexer<LexemeType, TokenType>,
    lexeme: &Result<LexemeType, LexemeType>,
) -> String {
    let span = lexeme.as_ref().unwrap().span();
    span_to_string(lexer, span)
}

pub fn lexeme_to_token(
    lexer: &dyn NonStreamingLexer<LexemeType, TokenType>,
    lexeme: Result<LexemeType, LexemeType>,
) -> Token {
    let lexeme = lexeme.unwrap();
    Token::new(lexeme.tok_id(), span_to_string(lexer, lexeme.span()))
}
