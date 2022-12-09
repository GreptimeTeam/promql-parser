use lrlex::{lrlex_mod, DefaultLexeme, LRNonStreamingLexer};
use lrpar::{lrpar_mod, Lexeme, NonStreamingLexer, Span};

lrlex_mod!("token_map");
use token_map::*;

pub fn lexer(s: &str) -> LRNonStreamingLexer<DefaultLexeme<u8>, u8> {
    let mut lexemes = Vec::new();
    let mut newlines = Vec::new();

    let lexeme = DefaultLexeme::new(T_NUMBER, 0, s.len());
    lexemes.push(Ok(lexeme));

    LRNonStreamingLexer::new(s, lexemes, newlines)
}
