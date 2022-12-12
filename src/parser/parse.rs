use crate::parser::{lex, Expr};

pub fn parse(input: &str) -> Result<Expr, String> {
    let lexer = lex::lexer(input);
    let (res, errs) = crate::promql_y::parse(&lexer);
    for err in errs {
        println!("{:?}", err)
    }

    res.unwrap()
}
