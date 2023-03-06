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

use crate::parser::{LexemeType, Token, TokenId};
use lrpar::{Lexeme, NonStreamingLexer, Span};

/// caller MUST pay attention to the index out of bounds issue
pub(crate) fn span_to_string(
    lexer: &dyn NonStreamingLexer<LexemeType, TokenId>,
    span: Span,
) -> String {
    lexer.span_str(span).to_string()
}

pub(crate) fn lexeme_to_string(
    lexer: &dyn NonStreamingLexer<LexemeType, TokenId>,
    lexeme: &Result<LexemeType, LexemeType>,
) -> Result<String, String> {
    lexeme
        .map(|l| span_to_string(lexer, l.span()))
        .map_err(|_| "ParseError".into())
}

pub(crate) fn lexeme_to_token(
    lexer: &dyn NonStreamingLexer<LexemeType, TokenId>,
    lexeme: Result<LexemeType, LexemeType>,
) -> Result<Token, String> {
    lexeme
        .map(|l| Token::new(l.tok_id(), span_to_string(lexer, l.span())))
        .map_err(|_| "ParseError".into())
}

// TODO: more test cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{lex, token};

    #[test]
    fn test_span_to_string() {
        let input = r#"prometheus_http_requests_total{code="200", job="prometheus"}"#;
        let span = Span::new(43, 43 + 3);
        let lexer = lex::lexer(input);
        assert!(lexer.is_ok());
        let span_str = span_to_string(&lexer.unwrap(), span);
        assert_eq!(span_str, "job");
    }

    #[test]
    fn test_lexeme_to_string() {
        let input = r#"prometheus_http_requests_total{code="200", job="prometheus"}"#;
        let lexeme = LexemeType::new(token::T_IDENTIFIER, 43, 3);
        let lexer = lex::lexer(input);
        assert!(lexer.is_ok());

        let lexeme_str = lexeme_to_string(&lexer.unwrap(), &Ok(lexeme));
        assert_eq!(lexeme_str, Ok(String::from("job")));
    }

    #[test]
    fn test_lexeme_to_token() {
        let input = r#"prometheus_http_requests_total{code="200", job="prometheus"}"#;
        let lexeme = LexemeType::new(token::T_IDENTIFIER, 43, 3);
        let lexer = lex::lexer(input);
        assert!(lexer.is_ok());
        let token = lexeme_to_token(&lexer.unwrap(), Ok(lexeme));
        assert_eq!(Ok(Token::new(token::T_IDENTIFIER, "job".into())), token);
    }
}
