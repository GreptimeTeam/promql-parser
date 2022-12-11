use super::LexemeType;
use super::{Token, TokenType};
use lrpar::{Lexeme, NonStreamingLexer, Span};

// caller MUST pay attention to the index out of bounds issue
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
