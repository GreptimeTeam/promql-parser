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
                println!("{err:?}")
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
    use crate::label::{MatchOp, Matcher, Matchers};
    use crate::parser::*;
    use crate::parser::{token, AtModifier as At};
    use crate::util::duration;
    use std::collections::HashSet;
    use std::time::Duration;

    enum Case {
        Success { input: String, expected: Expr },
        Fail { input: String, err_msg: String },
    }

    impl Case {
        fn new_success_case(input: String, expected: Expr) -> Self {
            Case::Success { input, expected }
        }
        fn new_fail_case(input: String, err_msg: String) -> Self {
            Case::Fail { input, err_msg }
        }

        fn new_success_cases(cases: Vec<(&str, Expr)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, expected)| Case::new_success_case(String::from(input), expected))
                .collect()
        }

        fn new_fail_cases(cases: Vec<(&str, &str)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, err_msg)| {
                    Case::new_fail_case(String::from(input), String::from(err_msg))
                })
                .collect()
        }
    }

    fn assert_cases(cases: Vec<Case>) {
        for case in cases {
            match case {
                Case::Success { input, expected } => {
                    let r = parse(&input);
                    assert!(r.is_ok(), "\n<parse> {input} failed, err {:?} ", r);
                    assert_eq!(r.unwrap(), expected, "\n<parse> {} does not match", input);
                }

                Case::Fail { input, err_msg } => {
                    let r = parse(&input);
                    assert!(
                        r.is_err(),
                        "\n<parse> '{input}' should failed, actually '{:?}' ",
                        r
                    );
                    assert_eq!(r.unwrap_err(), err_msg);
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

    #[test]
    #[ignore]
    fn test_vector_binary_expr_parser() {
        // "1 + 1"
        // "1 - 1"
        // "1 * 1"
        // "1 / 1"
        // "1 % 1"
        // "1 == bool 1"
        // "1 != bool 1"
        // "1 > bool 1"
        // "1 >= bool 1"
        // "1 < bool 1"
        // "1 <= bool 1"
        // "-1^2"
        // "-1*2"
        // "-1+2"
        // "-1^-2" // unary on binary expr
        // "+1 + -2 * 1"
        // "1 + 2/(3*1)"
        // "1 < bool 2 - 1 * 2"
        // "foo * bar"
        // "foo * sum"
        // "foo == 1"
        // "foo == bool 1"
        // "2.5 / bar"
        // "foo and bar"
        // "foo or bar"
        // "foo unless bar"
        // "foo + bar or bla and blub"
        // "foo and bar unless baz or qux"
        // "bar + on(foo) bla / on(baz, buz) group_right(test) blub"
        // "foo * on(test,blub) bar"
        // "foo * on(test,blub) group_left bar"
        // "foo and on(test,blub) bar"
        // "foo and on() bar"
        // "foo and ignoring(test,blub) bar"
        // "foo and ignoring() bar"
        // "foo unless on(bar) baz"
        // "foo / on(test,blub) group_left(bar) bar"
        // "foo / ignoring(test,blub) group_left(blub) bar"
        // "foo / ignoring(test,blub) group_left(bar) bar"
        // "foo - on(test,blub) group_right(bar,foo) bar"
        // "foo - ignoring(test,blub) group_right(bar,foo) bar"

        let fail_cases = vec![
            // (
            //     "foo and 1",
            //     "set operator \"and\" not allowed in binary scalar expression",
            // ),
            // (
            //     "1 and foo",
            //     "set operator \"and\" not allowed in binary scalar expression",
            // ),
            // (
            //     "foo or 1",
            //     "set operator \"or\" not allowed in binary scalar expression",
            // ),
            // (
            //     "1 or foo",
            //     "set operator \"or\" not allowed in binary scalar expression",
            // ),
            // (
            //     "foo unless 1",
            //     "set operator \"unless\" not allowed in binary scalar expression",
            // ),
            // (
            //     "1 unless foo",
            //     "set operator \"unless\" not allowed in binary scalar expression",
            // ),
            // (
            //     "1 or on(bar) foo",
            //     "vector matching only allowed between instant vectors",
            // ),
            // (
            //     "foo == on(bar) 10",
            //     "vector matching only allowed between instant vectors",
            // ),
            // ("foo + group_left(baz) bar", "unexpected <group_left>"),
            // (
            //     "foo and on(bar) group_left(baz) bar",
            //     "no grouping allowed for \"and\" operation",
            // ),
            // (
            //     "foo and on(bar) group_right(baz) bar",
            //     "no grouping allowed for \"and\" operation",
            // ),
            // (
            //     "foo or on(bar) group_left(baz) bar",
            //     "no grouping allowed for \"or\" operation",
            // ),
            // (
            //     "foo or on(bar) group_right(baz) bar",
            //     "no grouping allowed for \"or\" operation",
            // ),
            // (
            //     "foo unless on(bar) group_left(baz) bar",
            //     "no grouping allowed for \"unless\" operation",
            // ),
            // (
            //     "foo unless on(bar) group_right(baz) bar",
            //     "no grouping allowed for \"unless\" operation",
            // ),
            // (
            //     r#"http_requests(group="production"} + on(instance) group_left(job,instance) cpu_count(type="smp"}"#,
            //     "label \"instance\" must not occur in ON and GROUP clause at once",
            // ),
            // (
            //     "foo + bool bar",
            //     "bool modifier can only be used on comparison operators",
            // ),
            // (
            //     "foo + bool 10",
            //     "bool modifier can only be used on comparison operators",
            // ),
            // (
            //     "foo and bool 10",
            //     "bool modifier can only be used on comparison operators",
            // ),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    #[ignore]
    fn test_unary_expr_parser() {
        // "-some_metric"
        // "+some_metric"
        // " +some_metric"
    }

    #[test]
    fn test_vector_selector_parser() {
        let cases = vec![
            ("foo", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap()
            }),
            ("min", {
                let name = String::from("min");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap()
            }),
            ("foo offset 5m", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.offset_expr(Offset::Pos(Duration::from_secs(60 * 5))))
                    .unwrap()
            }),
            ("foo offset -7m", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let offset = Duration::from_secs(60 * 7);
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.offset_expr(Offset::Neg(offset)))
                    .unwrap()
            }),
            ("foo OFFSET 1h30m", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let offset = Duration::from_secs(60 * 90);

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.offset_expr(Offset::Pos(offset)))
                    .unwrap()
            }),
            ("foo OFFSET 1h30ms", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let offset = Duration::from_secs(60 * 60) + Duration::from_millis(30);

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.offset_expr(Offset::Pos(offset)))
                    .unwrap()
            }),
            ("foo @ 1603774568", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(1603774568f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ -100", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(-100f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ .3", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(0.3f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ 3.", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(3f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ 3.33", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(3.33f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ 3.3333", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                // Rounding off
                let at = At::try_from(3.333f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ 3.3335", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                // Rounding off
                let at = At::try_from(3.334f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ 3e2", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(300f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ 3e-1", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(0.3).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ 0xA", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(10f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            ("foo @ -3.3e1", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(-33f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
                    .unwrap()
            }),
            (r#"foo:bar{a="bc"}"#, {
                let name = String::from("foo:bar");
                let matchers = Matchers::new(HashSet::from([
                    Matcher::new_eq_metric_matcher(name.clone()),
                    Matcher::new(MatchOp::Equal, String::from("a"), String::from("bc")),
                ]));
                Expr::new_vector_selector(Some(name), matchers).unwrap()
            }),
            (r#"foo{NaN='bc'}"#, {
                let name = String::from("foo");
                let matchers = Matchers::new(HashSet::from([
                    Matcher::new_eq_metric_matcher(name.clone()),
                    Matcher::new(MatchOp::Equal, String::from("NaN"), String::from("bc")),
                ]));
                Expr::new_vector_selector(Some(name), matchers).unwrap()
            }),
            (r#"foo{bar='}'}"#, {
                let name = String::from("foo");
                let matchers = Matchers::new(HashSet::from([
                    Matcher::new_eq_metric_matcher(name.clone()),
                    Matcher::new(MatchOp::Equal, String::from("bar"), String::from("}")),
                ]));
                Expr::new_vector_selector(Some(name), matchers).unwrap()
            }),
            (r#"foo{a="b", foo!="bar", test=~"test", bar!~"baz"}"#, {
                let name = String::from("foo");
                let matchers = Matchers::new(HashSet::from([
                    Matcher::new_eq_metric_matcher(name.clone()),
                    Matcher::new(MatchOp::Equal, String::from("a"), String::from("b")),
                    Matcher::new(MatchOp::NotEqual, String::from("foo"), String::from("bar")),
                    Matcher::new_matcher(
                        token::T_EQL_REGEX,
                        String::from("test"),
                        String::from("test"),
                    )
                    .unwrap(),
                    Matcher::new_matcher(
                        token::T_NEQ_REGEX,
                        String::from("bar"),
                        String::from("baz"),
                    )
                    .unwrap(),
                ]));
                Expr::new_vector_selector(Some(name), matchers).unwrap()
            }),
            (r#"foo{a="b", foo!="bar", test=~"test", bar!~"baz",}"#, {
                let name = String::from("foo");
                let matchers = Matchers::new(HashSet::from([
                    Matcher::new_eq_metric_matcher(name.clone()),
                    Matcher::new(MatchOp::Equal, String::from("a"), String::from("b")),
                    Matcher::new(MatchOp::NotEqual, String::from("foo"), String::from("bar")),
                    Matcher::new_matcher(
                        token::T_EQL_REGEX,
                        String::from("test"),
                        String::from("test"),
                    )
                    .unwrap(),
                    Matcher::new_matcher(
                        token::T_NEQ_REGEX,
                        String::from("bar"),
                        String::from("baz"),
                    )
                    .unwrap(),
                ]));
                Expr::new_vector_selector(Some(name), matchers).unwrap()
            }),
        ];
        assert_cases(Case::new_success_cases(cases));

        let fail_cases = vec![
            ("foo @ +Inf", "timestamp out of bounds for @ modifier: inf"),
            ("foo @ -Inf", "timestamp out of bounds for @ modifier: -inf"),
            ("foo @ NaN", "timestamp out of bounds for @ modifier: NaN"),
            ("{", "unexpected end of input inside braces"),
            ("}", "unexpected right brace '}'"),
            ("some{", "unexpected end of input inside braces"),
            ("some}", "unexpected right brace '}'"),
            (
                "some_metric{a=b}",
                "unexpected identifier \"b\" in label matching, expected string",
            ),
            // (
            //     r#"some_metric{a:b="b"}"#,
            //     "unexpected character inside braces: ':'",
            // ),
            // (r#"foo{a*"b"}"#, "unexpected character inside braces: '*'"),
            // // (
            // //               r#"foo{a>="b"}"#,
            // //              // TODO(fabxc): willingly lexing wrong tokens allows for more precise error
            // //              // messages from the parser - consider if this is an option. "unexpected character inside braces: '>'",
            // // ),
            // (
            //     r#"some_metric{a=\"\xff\"}"#,
            //     "1:15: parse error: invalid UTF-8 rune",
            // ),
            // (
            //     "foo{gibberish}",
            //     r#"unexpected "}" in label matching, expected label matching operator"#,
            // ),
            // ("foo{1}", "unexpected character inside braces: '1'"),
            // (
            //     "{}",
            //     "vector selector must contain at least one non-empty matcher",
            // ),
            // (
            //     r#"{x=""}"#,
            //     "vector selector must contain at least one non-empty matcher",
            // ),
            // (
            //     r#"{x=~".*"}"#,
            //     "vector selector must contain at least one non-empty matcher",
            // ),
            // (
            //     r#"{x!~".+"}"#,
            //     "vector selector must contain at least one non-empty matcher",
            // ),
            // (
            //     r#"{x!="a"}"#,
            //     "vector selector must contain at least one non-empty matcher",
            // ),
            // (
            //     r#"foo{__name__="bar"}"#,
            //     r#"metric name must not be set twice: "foo" or "bar""#,
            // ),
            // (
            //     "foo{__name__= =}",
            //     r#"1:15: parse error: unexpected "=" in label matching, expected string"#,
            // ),
            // (
            //     "foo{,}",
            //     r#"unexpected "," in label matching, expected identifier or "}""#,
            // ),
            // (
            //     r#"foo{__name__ == "bar"}"#,
            //     r#"1:15: parse error: unexpected "=" in label matching, expected string"#,
            // ),
            // (
            //     r#"foo{__name__="bar" lol}"#,
            //     r#"unexpected identifier "lol" in label matching, expected "," or "}""#,
            // ),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));

        let fail_cases = vec![
            {
                let num = f64::MAX - 1f64;
                let input = format!("foo @ {num}");
                let err_msg = format!("timestamp out of bounds for @ modifier: {num}");
                Case::Fail { input, err_msg }
            },
            {
                let num = f64::MIN - 1f64;
                let input = format!("foo @ {num}");
                let err_msg = format!("timestamp out of bounds for @ modifier: {num}");
                Case::Fail { input, err_msg }
            },
        ];
        assert_cases(fail_cases);
    }

    #[test]
    fn test_matrix_selector() {
        let cases = vec![
            ("test[5s]", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, Duration::from_secs(5)))
                    .unwrap()
            }),
            ("test[5m]", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, duration::MINUTE_DURATION * 5))
                    .unwrap()
            }),
            ("test[5m30s]", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, Duration::from_secs(330)))
                    .unwrap()
            }),
            ("test[5h] OFFSET 5m", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, duration::HOUR_DURATION * 5))
                    .and_then(|ms| ms.offset_expr(Offset::Pos(duration::MINUTE_DURATION * 5)))
                    .unwrap()
            }),
            ("test[5d] OFFSET 10s", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, duration::DAY_DURATION * 5))
                    .and_then(|ms| ms.offset_expr(Offset::Pos(Duration::from_secs(10))))
                    .unwrap()
            }),
            ("test[5w] offset 2w", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, duration::WEEK_DURATION * 5))
                    .and_then(|ms| ms.offset_expr(Offset::Pos(duration::WEEK_DURATION * 2)))
                    .unwrap()
            }),
            (r#"test{a="b"}[5y] OFFSET 3d"#, {
                let name = String::from("test");
                let name_matcher = Matcher::new_eq_metric_matcher(name.clone());
                let label_matcher =
                    Matcher::new(MatchOp::Equal, String::from("a"), String::from("b"));
                Expr::new_vector_selector(
                    Some(name),
                    Matchers::new(HashSet::from([name_matcher, label_matcher])),
                )
                .and_then(|vs| Expr::new_matrix_selector(vs, duration::YEAR_DURATION * 5))
                .and_then(|ms| ms.offset_expr(Offset::Pos(duration::DAY_DURATION * 3)))
                .unwrap()
            }),
            (r#"test{a="b"}[5y] @ 1603774699"#, {
                let name = String::from("test");
                let name_matcher = Matcher::new_eq_metric_matcher(name.clone());
                let label_matcher =
                    Matcher::new(MatchOp::Equal, String::from("a"), String::from("b"));
                Expr::new_vector_selector(
                    Some(name),
                    Matchers::new(HashSet::from([name_matcher, label_matcher])),
                )
                .and_then(|vs| Expr::new_matrix_selector(vs, duration::YEAR_DURATION * 5))
                .and_then(|ms| ms.step_invariant_expr(At::try_from(1603774699_f64).unwrap()))
                .unwrap()
            }),
        ];

        assert_cases(Case::new_success_cases(cases));

        // TODO: fulfil these failure cases
        let fail_cases = vec![
            ("foo[5mm]", "bad duration syntax: 5mm"),
            ("foo[5m1]", "bad duration syntax: 5m1]"),
            ("foo[5m:1m1]", "bad duration syntax: 1m1]"),
            ("foo[5y1hs]", "not a valid duration string: 5y1hs"),
            ("foo[5m1h]", "not a valid duration string: 5m1h"),
            ("foo[5m1m]", "not a valid duration string: 5m1m"),
            ("foo[0m]", "duration must be greater than 0"),
            (r#"foo["5m"]"#, r#"unexpected character inside brackets: ""#),
            (r#"foo[]"#, r#"empty duration string"#),
            (r#"foo[1]"#, r#"bad duration syntax: 1]"#),
            // ("some_metric[5m] OFFSET 1", ""),
            (
                "some_metric[5m] OFFSET 1mm",
                "bad number or duration syntax: 1mm",
            ),
            // ("some_metric[5m] OFFSET", ""),
            (
                "some_metric OFFSET 1m[5m]",
                "no offset modifiers allowed before range",
            ),
            // ("some_metric[5m] @ 1m", ""),
            // ("some_metric[5m] @", ""),
            (
                "some_metric @ 1234 [5m]",
                "no @ modifiers allowed before range",
            ),
            // ("(foo + bar)[5m]", ""),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    fn test_aggregation_expr_parser() {
        let cases = vec![
            ("sum by (foo) (some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("avg by (foo)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_AVG, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("max by (foo)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_MAX, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("sum without (foo) (some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::Without(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("sum (some_metric) without (foo)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::Without(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("stddev(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_STDDEV, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("stdvar by (foo)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_STDVAR, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("sum by ()(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("sum by (foo,bar,)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching =
                    AggModifier::By(HashSet::from([String::from("foo"), String::from("bar")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("sum by (foo,)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
                    .unwrap()
            }),
            ("topk(5, some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                let param = Expr::new_number_literal(5.0).unwrap();
                let args = FunctionArgs::new_args(param).append_args(vs);
                Expr::new_aggregate_expr(token::T_TOPK, matching, args).unwrap()
            }),
            (r#"count_values("value", some_metric)"#, {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                let param = Expr::new_string_literal("value".into()).unwrap();
                let args = FunctionArgs::new_args(param).append_args(vs);
                Expr::new_aggregate_expr(token::T_COUNT_VALUES, matching, args).unwrap()
            }),
            (
                "sum without(and, by, avg, count, alert, annotations)(some_metric)",
                {
                    let name = String::from("some_metric");
                    let matcher = Matcher::new_eq_metric_matcher(name.clone());
                    let matching = AggModifier::Without(
                        vec!["and", "by", "avg", "count", "alert", "annotations"]
                            .into_iter()
                            .map(String::from)
                            .collect(),
                    );
                    let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                    Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
                        .unwrap()
                },
            ),
        ];

        assert_cases(Case::new_success_cases(cases));

        // TODO: fulfil these failure cases
        let fail_cases = vec![
            // ("sum without(==)(some_metric)", ""),
            // ("sum without(,)(some_metric)", ""),
            // ("sum without(foo,,)(some_metric)", ""),
            // ("sum some_metric by (test)", ""),
            // ("sum (some_metric) by test", ""),
            // ("sum () by (test)", ""),
            // ("MIN keep_common (some_metric)", ""),
            // ("MIN (some_metric) keep_common", ""),
            // ("sum without (test) (some_metric) by (test)", ""),
            // ("topk(some_metric)", ""),
            // ("topk(some_metric,)", ""),
            // ("topk(some_metric, other_metric)", ""),
            // ("count_values(5, other_metric)", ""),
            // ("rate(some_metric[5m]) @ 1234", ""),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    // TODO: fulfil function call cases
    #[test]
    #[ignore]
    fn test_function_call_parser() {}

    // TODO: fulfil subquery cases
    #[test]
    #[ignore]
    fn test_subquery_parser() {}

    // TODO: fulfil these failure cases
    #[test]
    fn test_fail_cases() {
        let fail_cases = vec![
            ("", "no expression found in input: ''"),
            (
                "# just a comment\n\n",
                "no expression found in input: '# just a comment\n\n'",
            ),
            // ("1+", "unexpected end of input"),
            (".", "unexpected character: '.'"),
            ("2.5.", "bad number or duration syntax: 2.5."),
            ("100..4", "bad number or duration syntax: 100.."),
            ("0deadbeef", "bad number or duration syntax: 0de"),
            // ("1 /", "unexpected end of input"),
            // ("*1", "unexpected <op:*>"),
            // ("(1))", "unexpected right parenthesis ')'"),
            // ("((1)", "unclosed left parenthesis"),
            // ("999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999", "out of range"),
            // ("(", "unclosed left parenthesis"),
            // ("1 and 1", "set operator \"and\" not allowed in binary scalar expression"),
            // ("1 == 1", "1:3: parse error: comparisons between scalars must use BOOL modifier"),
            // ("1 or 1", "set operator \"or\" not allowed in binary scalar expression"),
            // ("1 unless 1", "set operator \"unless\" not allowed in binary scalar expression"),
            // ("1 !~ 1", `unexpected character after '!': '~'`),
            // ("1 =~ 1", `unexpected character after '=': '~'`),
            // (`-"string"`, `unary expression only allowed on expressions of type scalar or instant vector, got "string"`),
            // (`-test[5m]`, `unary expression only allowed on expressions of type scalar or instant vector, got "range vector"`),
            // ("*test", "unexpected <op:*>"),
            // ("1 offset 1d", "1:1: parse error: offset modifier must be preceded by an instant vector selector or range vector selector or a subquery"),
            // (
            //     "foo offset 1s offset 2s",
            //     "offset may not be set multiple times",
            // ),
            // (
            //     "a - on(b) ignoring(c) d",
            //     "1:11: parse error: unexpected <ignoring>",
            // ),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }
}
