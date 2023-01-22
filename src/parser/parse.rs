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

use crate::parser::{lex, Expr};

pub fn parse(input: &str) -> Result<Expr, String> {
    match lex::lexer(input) {
        Err(e) => Err(e),
        Ok(lexer) => {
            let (res, errs) = crate::promql_y::parse(&lexer);
            for err in errs {
                println!("{:?}", err)
            }
            match res {
                Some(r) => check_ast(r),
                None => Err("empty AST".into()),
            }
        }
    }
}

// TODO: check the validation of the expr
// https://github.com/prometheus/prometheus/blob/0372e259baf014bbade3134fd79bcdfd8cbdef2c/promql/parser/parse.go#L436
fn check_ast(expr: Result<Expr, String>) -> Result<Expr, String> {
    expr
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Case {
        input: &'static str,   // The input to be parsed.
        expected: Expr,        // The expected expression AST.
        fail: bool,            // Whether parsing is supposed to fail.
        err_msg: &'static str, // If not empty the parsing error has to contain this string.
    }

    impl Case {
        fn new((input, expected, fail, err_msg): (&'static str, Expr, bool, &'static str)) -> Self {
            Self {
                input,
                expected,
                fail,
                err_msg,
            }
        }
    }

    // Scalars and scalar-to-scalar operations.
    // #[test]
    // fn test_valid_scalar_operation_parser() {
    //     let cases: Vec<Case> = vec![
    //         ("1", NumberLiteral::new(1.0), false, ""),
    //         ("+Inf", NumberLiteral::new(1.0), false, ""),
    //         ("-Inf", NumberLiteral::new(f64::NEG_INFINITY), false, ""),
    //         (".5", NumberLiteral::new(0.5), false, ""),
    //         ("5.", NumberLiteral::new(5.0), false, ""),
    //         ("123.4567", NumberLiteral::new(123.4567), false, ""),
    //         ("5e-3", NumberLiteral::new(0.005), false, ""),
    //         ("5e3", NumberLiteral::new(5000.0), false, ""),
    //         ("0xc", NumberLiteral::new(12.0), false, ""),
    //         ("0755", NumberLiteral::new(493.0), false, ""),
    //         ("+5.5e-3", NumberLiteral::new(0.0055), false, ""),
    //         ("-0755", NumberLiteral::new(-493.0), false, ""),
    //     ]
    //     .into_iter()
    //     .map(Case::new)
    //     .collect();

    //     for case in cases {
    //         let Case {
    //             input,
    //             expected,
    //             fail,
    //             err_msg,
    //         } = case;

    //         let r = parse(input);
    //         if !fail {
    //             assert!(r.is_ok(), "{:?} is not ok", r);
    //             // match r.unwrap() {
    //             //     Expr::NumberLiteral(nl) => assert_eq!(expected, nl, "{} does not match", input),
    //             //     _ => {}
    //             // }
    //         } else {
    //             let err = r.unwrap_err();
    //             assert!(
    //                 &err.contains(err_msg),
    //                 "{:?} does not contains {}",
    //                 &err,
    //                 err_msg
    //             );
    //         }
    //     }
    // }
}
