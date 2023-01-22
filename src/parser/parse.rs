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

/// cases in original prometheus is a huge slices which are constructed more than 3000 lines,
/// and it is hard to split them based on the original order. So here is the Note:
///
/// - all cases SHOULD be covered, and the same literal float and literal
///   string SHOULD be the same with the original prometheus.
/// - all cases will be splitted into different blocks based on the type of parsed Expr.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::*;

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

    fn assert_cases(cases: Vec<Case>) {
        for Case {
            input,
            expected,
            fail,
            err_msg,
        } in cases
        {
            let r = parse(input);
            if !fail {
                assert!(r.is_ok(), "{:?} is not ok", r);
                assert_eq!(expected, r.unwrap(), "{} does not match", input);
            } else {
                let err = r.unwrap_err();
                assert!(
                    &err.contains(err_msg),
                    "{:?} does not contains {}",
                    &err,
                    err_msg
                );
            }
        }
    }

    #[test]
    fn test_number_literal_parser() {
        let cases: Vec<Case> = vec![
            ("1", Expr::new_number_literal(1.0).unwrap(), false, ""),
            (
                "+Inf",
                Expr::new_number_literal(f64::INFINITY).unwrap(),
                false,
                "",
            ),
            // (
            //     "-Inf",
            //     Expr::new_number_literal(f64::NEG_INFINITY).unwrap(),
            //     false,
            //     "",
            // ),
            (".5", Expr::new_number_literal(0.5).unwrap(), false, ""),
            ("5.", Expr::new_number_literal(5.0).unwrap(), false, ""),
            (
                "123.4567",
                Expr::new_number_literal(123.4567).unwrap(),
                false,
                "",
            ),
            ("5e-3", Expr::new_number_literal(0.005).unwrap(), false, ""),
            ("5e3", Expr::new_number_literal(5000.0).unwrap(), false, ""),
            // ("0xc", Expr::new_number_literal(12.0).unwrap(), false, ""),
            // ("0755", Expr::new_number_literal(493.0).unwrap(), false, ""),
            (
                "+5.5e-3",
                Expr::new_number_literal(0.0055).unwrap(),
                false,
                "",
            ),
            // (
            //     "-0755",
            //     Expr::new_number_literal(-493.0).unwrap(),
            //     false,
            //     "",
            // ),
        ]
        .into_iter()
        .map(Case::new)
        .collect();

        assert_cases(cases);
    }
}
