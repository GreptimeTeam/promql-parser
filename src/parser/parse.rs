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

    enum Case {
        Success {
            input: &'static str,
            expected: Expr,
        },
        Fail {
            input: &'static str,
            err_msg: &'static str,
        },
    }

    impl Case {
        fn new_success_case(input: &'static str, expected: Expr) -> Self {
            Case::Success { input, expected }
        }
        fn new_fail_case(input: &'static str, err_msg: &'static str) -> Self {
            Case::Fail { input, err_msg }
        }

        fn new_success_cases(cases: Vec<(&'static str, Expr)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, expected)| Case::new_success_case(input, expected))
                .collect()
        }

        fn new_fail_cases(cases: Vec<(&'static str, &'static str)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, err_msg)| Case::new_fail_case(input, err_msg))
                .collect()
        }
    }

    fn assert_cases(cases: Vec<Case>) {
        for case in cases {
            match case {
                Case::Success { input, expected } => {
                    let r = parse(input);
                    assert!(r.is_ok(), "parse {input} failed, err {:?} ", r);
                    assert_eq!(
                        r.unwrap(),
                        expected,
                        "parse {} does not match, expected: {:?}",
                        input,
                        expected
                    );
                }

                Case::Fail { input, err_msg } => {
                    let r = parse(input);
                    assert!(r.is_err(), "parse {input} should failed, actually {:?} ", r);
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
    }

    #[test]
    fn test_number_literal_parser() {
        let cases = vec![
            ("1", Expr::new_number_literal(1.0).unwrap()),
            ("+Inf", Expr::new_number_literal(f64::INFINITY).unwrap()),
            ("-Inf", Expr::new_number_literal(f64::NEG_INFINITY).unwrap()),
            (".5", Expr::new_number_literal(0.5).unwrap()),
            ("5.", Expr::new_number_literal(5.0).unwrap()),
            ("123.4567", Expr::new_number_literal(123.4567).unwrap()),
            ("5e-3", Expr::new_number_literal(0.005).unwrap()),
            ("5e3", Expr::new_number_literal(5000.0).unwrap()),
            ("0xc", Expr::new_number_literal(12.0).unwrap()),
            ("0755", Expr::new_number_literal(493.0).unwrap()),
            ("+5.5e-3", Expr::new_number_literal(0.0055).unwrap()),
            ("-0755", Expr::new_number_literal(-493.0).unwrap()),
        ];

        assert_cases(Case::new_success_cases(cases));
    }
}
