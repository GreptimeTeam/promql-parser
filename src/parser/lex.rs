use lrlex::{lrlex_mod, DefaultLexeme, LRNonStreamingLexer};
use lrpar::Lexeme;

lrlex_mod!("token_map");
use token_map::*;

pub type StorageType = u8;
pub type LexemeType = DefaultLexeme<StorageType>;
pub type Lexer<'a> = LRNonStreamingLexer<'a, 'a, LexemeType, StorageType>;

pub fn lexer(s: &str) -> Lexer {
    let mut lexemes = Vec::new();

    // let lexeme = DefaultLexeme::new(T_STRING, 0, s.len());
    let lexeme = DefaultLexeme::new(T_DURATION, 0, s.len());
    lexemes.push(Ok(lexeme));

    LRNonStreamingLexer::new(s, lexemes, Vec::new())
}
