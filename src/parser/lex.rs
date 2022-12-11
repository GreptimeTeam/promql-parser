use super::TokenType;
use lrlex::{DefaultLexeme, LRNonStreamingLexer};

pub type LexemeType = DefaultLexeme<TokenType>;
pub type Lexer<'a> = LRNonStreamingLexer<'a, 'a, LexemeType, TokenType>;

pub fn lexer(s: &str) -> Lexer {
    let mut lexemes = Vec::new();

    // let lexeme = DefaultLexeme::new(T_STRING, 0, s.len());
    // let lexeme = DefaultLexeme::new(T_DURATION, 0, s.len());
    // lexemes.push(Ok(lexeme));

    LRNonStreamingLexer::new(s, lexemes, Vec::new())
}
