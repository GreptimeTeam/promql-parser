use lrpar::{NonStreamingLexer, Span, Lexeme};
use super::lex::{LexemeType, StorageType};

// caller MUST pay attention to the index out of bounds issue
pub fn span_to_string(
    lexer: &dyn NonStreamingLexer<LexemeType, StorageType>,
    span: Span,
) -> Result<String, String> {
    Ok(lexer.span_str(span).to_string())
}

// = note: expected reference `&lrlex::LRNonStreamingLexer<'_, '_, DefaultLexeme<u8>, u8>`
// found reference `&'lexer (dyn NonStreamingLexer<'input, DefaultLexeme<u8>, u8> + 'lexer)`

pub fn lexeme_to_string(
    lexer: &dyn NonStreamingLexer<LexemeType, StorageType>,
    lexeme: &Result<LexemeType, LexemeType>,
) -> Result<String, String> {
    let span = lexeme.as_ref().unwrap().span();
    Ok(lexer.span_str(span).to_string())
}
