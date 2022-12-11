use super::TokenType;
use lrlex::{DefaultLexeme, LRNonStreamingLexer};
use lrpar::Lexeme;

pub type LexemeType = DefaultLexeme<TokenType>;
pub type Lexer<'a> = LRNonStreamingLexer<'a, 'a, LexemeType, TokenType>;

// FIXME: this is just a demo Lexer, constructed for example
pub fn lexer(s: &str) -> Lexer {
    let mut start = 0;
    let mut len = "node_cpu_seconds_total".len();
    let metric_identifier_lexeme = DefaultLexeme::new(super::T_METRIC_IDENTIFIER, start, len);

    start += len;
    len = "{".len();
    let left_brace_lexeme = DefaultLexeme::new(super::T_LEFT_BRACE, start, len);

    start += len;
    len = "cpu".len();
    let identifier1_lexeme = DefaultLexeme::new(super::T_IDENTIFIER, start, len);

    start += len;
    len = "=".len();
    let eql1_lexeme = DefaultLexeme::new(super::T_EQL, start, len);

    start += len;
    len = "0".len();
    let val1_lexeme = DefaultLexeme::new(super::T_STRING, start, len);

    start += len;
    len = ",".len();
    let comma_lexeme = DefaultLexeme::new(super::T_COMMA, start, len);

    start += len;
    len = "mode".len();
    let identifier2_lexeme = DefaultLexeme::new(super::T_IDENTIFIER, start, len);

    start += len;
    len = "=".len();
    let eql2_lexeme = DefaultLexeme::new(super::T_EQL, start, len);

    start += len;
    len = "idel".len();
    let val2_lexeme = DefaultLexeme::new(super::T_STRING, start, len);

    start += len;
    len = "}".len();
    let right_brace_lexeme = DefaultLexeme::new(super::T_RIGHT_BRACE, start, len);

    let lexemes = vec![
        metric_identifier_lexeme,
        left_brace_lexeme,
        identifier1_lexeme,
        eql1_lexeme,
        val1_lexeme,
        comma_lexeme,
        identifier2_lexeme,
        eql2_lexeme,
        val2_lexeme,
        right_brace_lexeme,
    ]
    .into_iter()
    .map(|l| Ok(l))
    .collect();

    LRNonStreamingLexer::new(s, lexemes, Vec::new())
}
