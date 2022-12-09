use lrlex::{lrlex_mod, DefaultLexeme, LRNonStreamingLexer};
use lrpar::{lrpar_mod, Lexeme, NonStreamingLexer, Span};

use super::{lex, Expr};

lrpar_mod!("parser/promql.y");

pub fn parse(input: &str) -> Result<Expr, String> {
    let lexer = lex::lexer(input);
    let (res, _errs) = promql_y::parse(&lexer);
    match res.unwrap() {
        Ok(expr) => {
            println!("{:?}", expr);
            Ok(expr)
        }
        e => e,
    }
}
