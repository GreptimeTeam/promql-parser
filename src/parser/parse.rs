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
                Some(r) => r,
                None => Err("empty AST".into()),
            }
        }
    }
}

// TODO: check the validation of the expr
// https://github.com/prometheus/prometheus/blob/0372e259baf014bbade3134fd79bcdfd8cbdef2c/promql/parser/parse.go#L436
#[allow(dead_code)]
fn check_ast(_expr: Expr) -> Result<Expr, String> {
    todo!();
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
    use crate::parser::{token, AggModifier, AtModifier as At, Expr, FunctionArgs, Offset};
    use crate::util::duration;
    use std::collections::HashSet;
    use std::time::Duration;

    struct Case {
        input: String,
        expected: Result<Expr, String>,
    }

    impl Case {
        fn new(input: String, expected: Result<Expr, String>) -> Self {
            Case { input, expected }
        }

        fn new_result_cases(cases: Vec<(&str, Result<Expr, String>)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, expected)| Case::new(String::from(input), expected))
                .collect()
        }

        fn new_expr_cases(cases: Vec<(&str, Expr)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, expected)| Case::new(String::from(input), Ok(expected)))
                .collect()
        }

        fn new_fail_cases(cases: Vec<(&str, &str)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, expected)| Case::new(String::from(input), Err(expected.into())))
                .collect()
        }
    }

    fn assert_cases(cases: Vec<Case>) {
        for Case { input, expected } in cases {
            assert_eq!(
                crate::parser::parse(&input),
                expected,
                "\n<parse> <{input:?}> does not match"
            );
        }
    }

    #[test]
    fn test_number_literal_parser() {
        let cases = vec![
            ("1", Expr::from(1.0)),
            ("+Inf", Expr::from(f64::INFINITY)),
            ("-Inf", Expr::from(f64::NEG_INFINITY)),
            (".5", Expr::from(0.5)),
            ("5.", Expr::from(5.0)),
            ("123.4567", Expr::from(123.4567)),
            ("5e-3", Expr::from(0.005)),
            ("5e3", Expr::from(5000.0)),
            ("0xc", Expr::from(12.0)),
            ("0755", Expr::from(493.0)),
            ("+5.5e-3", Expr::from(0.0055)),
            ("-0755", Expr::from(-493.0)),
        ];
        assert_cases(Case::new_expr_cases(cases));
    }

    #[test]
    fn test_string_literal_parser() {
        let cases = vec![
            (
                "\"double-quoted string \\\" with escaped quote\"",
                Expr::from("double-quoted string \\\" with escaped quote"),
            ),
            (
                // this case is the same with the previous one
                r#""double-quoted string \" with escaped quote""#,
                Expr::from(r#"double-quoted string \" with escaped quote"#),
            ),
            (
                r#"'single-quoted string \' with escaped quote'"#,
                Expr::from(r#"single-quoted string \' with escaped quote"#),
            ),
            (
                "`backtick-quoted string`",
                Expr::from("backtick-quoted string"),
            ),
            // "\a\b\f\n\r\t\v\\\" - \xFF\377\u1234\U00010111\U0001011111☺"
            // '\a\b\f\n\r\t\v\\\' - \xFF\377\u1234\U00010111\U0001011111☺'
            // "`" + `\a\b\f\n\r\t\v\\\"\' - \xFF\377\u1234\U00010111\U0001011111☺` + "`"
        ];
        assert_cases(Case::new_expr_cases(cases));

        let fail_cases = vec![
            // "`\\``"
            // `"\`
            // `"\c"`
            // `"\x."`
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
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
        // "a + sum"

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
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
            }),
            ("min", {
                let name = String::from("min");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
            }),
            ("foo offset 5m", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.offset_expr(Offset::Pos(Duration::from_secs(60 * 5))))
            }),
            ("foo offset -7m", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let offset = Duration::from_secs(60 * 7);
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.offset_expr(Offset::Neg(offset)))
            }),
            ("foo OFFSET 1h30m", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let offset = Duration::from_secs(60 * 90);

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.offset_expr(Offset::Pos(offset)))
            }),
            ("foo OFFSET 1h30ms", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let offset = Duration::from_secs(60 * 60) + Duration::from_millis(30);

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.offset_expr(Offset::Pos(offset)))
            }),
            ("foo @ 1603774568", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(1603774568f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ -100", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(-100f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ .3", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(0.3f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ 3.", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(3f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ 3.33", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(3.33f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ 3.3333", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                // Rounding off
                let at = At::try_from(3.333f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ 3.3335", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                // Rounding off
                let at = At::try_from(3.334f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ 3e2", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(300f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ 3e-1", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(0.3).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ 0xA", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(10f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            ("foo @ -3.3e1", {
                let name = String::from("foo");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let at = At::try_from(-33f64).unwrap();

                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|e| e.step_invariant_expr(at))
            }),
            (r#"foo:bar{a="bc"}"#, {
                let name = String::from("foo:bar");
                let matchers = Matchers::new(HashSet::from([
                    Matcher::new_eq_metric_matcher(name.clone()),
                    Matcher::new(MatchOp::Equal, String::from("a"), String::from("bc")),
                ]));
                Expr::new_vector_selector(Some(name), matchers)
            }),
            (r#"foo{NaN='bc'}"#, {
                let name = String::from("foo");
                let matchers = Matchers::new(HashSet::from([
                    Matcher::new_eq_metric_matcher(name.clone()),
                    Matcher::new(MatchOp::Equal, String::from("NaN"), String::from("bc")),
                ]));
                Expr::new_vector_selector(Some(name), matchers)
            }),
            (r#"foo{bar='}'}"#, {
                let name = String::from("foo");
                let matchers = Matchers::new(HashSet::from([
                    Matcher::new_eq_metric_matcher(name.clone()),
                    Matcher::new(MatchOp::Equal, String::from("bar"), String::from("}")),
                ]));
                Expr::new_vector_selector(Some(name), matchers)
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
                Expr::new_vector_selector(Some(name), matchers)
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
                Expr::new_vector_selector(Some(name), matchers)
            }),
        ];
        assert_cases(Case::new_result_cases(cases));

        let fail_cases = vec![
            ("foo @ +Inf", "timestamp out of bounds for @ modifier: inf"),
            ("foo @ -Inf", "timestamp out of bounds for @ modifier: -inf"),
            ("foo @ NaN", "timestamp out of bounds for @ modifier: NaN"),
            ("{", "unexpected end of input inside braces"),
            ("}", "unexpected right brace '}'"),
            ("some{", "unexpected end of input inside braces"),
            ("some}", "unexpected right brace '}'"),
            // (
            //     "some_metric{a=b}",
            //     "unexpected identifier \"b\" in label matching, expected string",
            // ),
            (
                r#"some_metric{a:b="b"}"#,
                "unexpected character inside braces: ':'",
            ),
            (r#"foo{a*"b"}"#, "unexpected character inside braces: '*'"),
            (r#"foo{a>="b"}"#, "unexpected character inside braces: '>'"),
            // (
            //     r#"some_metric{a="\xff"}"#,
            //     "1:15: parse error: invalid UTF-8 rune",
            // ),
            // (
            //     "foo{gibberish}",
            //     r#"unexpected "}" in label matching, expected label matching operator"#,
            // ),
            ("foo{1}", "unexpected character inside braces: '1'"),
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
                let expected = Err(format!("timestamp out of bounds for @ modifier: {num}"));
                Case { input, expected }
            },
            {
                let num = f64::MIN - 1f64;
                let input = format!("foo @ {num}");
                let expected = Err(format!("timestamp out of bounds for @ modifier: {num}"));
                Case { input, expected }
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
            }),
            ("test[5m]", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, duration::MINUTE_DURATION * 5))
            }),
            ("test[5m30s]", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, Duration::from_secs(330)))
            }),
            ("test[5h] OFFSET 5m", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, duration::HOUR_DURATION * 5))
                    .and_then(|ms| ms.offset_expr(Offset::Pos(duration::MINUTE_DURATION * 5)))
            }),
            ("test[5d] OFFSET 10s", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, duration::DAY_DURATION * 5))
                    .and_then(|ms| ms.offset_expr(Offset::Pos(Duration::from_secs(10))))
            }),
            ("test[5w] offset 2w", {
                let name = String::from("test");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                Expr::new_vector_selector(Some(name), Matchers::one(matcher))
                    .and_then(|vs| Expr::new_matrix_selector(vs, duration::WEEK_DURATION * 5))
                    .and_then(|ms| ms.offset_expr(Offset::Pos(duration::WEEK_DURATION * 2)))
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
            }),
        ];

        assert_cases(Case::new_result_cases(cases));

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
            }),
            ("avg by (foo)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_AVG, matching, FunctionArgs::new_args(vs))
            }),
            ("max by (foo)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_MAX, matching, FunctionArgs::new_args(vs))
            }),
            ("sum without (foo) (some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::Without(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
            }),
            ("sum (some_metric) without (foo)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::Without(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
            }),
            ("stddev(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_STDDEV, matching, FunctionArgs::new_args(vs))
            }),
            ("stdvar by (foo)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_STDVAR, matching, FunctionArgs::new_args(vs))
            }),
            ("sum by ()(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
            }),
            ("sum by (foo,bar,)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching =
                    AggModifier::By(HashSet::from([String::from("foo"), String::from("bar")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
            }),
            ("sum by (foo,)(some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::from([String::from("foo")]));
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
            }),
            ("topk(5, some_metric)", {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                let param = Expr::from(5.0);
                let args = FunctionArgs::new_args(param).append_args(vs);
                Expr::new_aggregate_expr(token::T_TOPK, matching, args)
            }),
            (r#"count_values("value", some_metric)"#, {
                let name = String::from("some_metric");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                let param = Expr::from("value");
                let args = FunctionArgs::new_args(param).append_args(vs);
                Expr::new_aggregate_expr(token::T_COUNT_VALUES, matching, args)
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
                },
            ),
            ("sum(sum)", {
                let name = String::from("sum");
                let matcher = Matcher::new_eq_metric_matcher(name.clone());
                let matching = AggModifier::By(HashSet::new());
                let vs = Expr::new_vector_selector(Some(name), Matchers::one(matcher)).unwrap();
                Expr::new_aggregate_expr(token::T_SUM, matching, FunctionArgs::new_args(vs))
            }),
        ];

        assert_cases(Case::new_result_cases(cases));

        let fail_cases = vec![
            // ("sum without(==)(some_metric)", ""),
            // ("sum without(,)(some_metric)", ""),
            // ("sum without(foo,,)(some_metric)", ""),
            // ("sum some_metric by (test)", ""),
            // ("sum (some_metric) by test", ""),
            // ("sum () by (test)", ""),
            // ("MIN keep_common (some_metric)", ""),
            // ("MIN (some_metric) keep_common", ""),
            // ("sum (some_metric) without (test) by (test)", ""),
            // ("sum without (test) (some_metric) by (test)", ""),
            // ("topk(some_metric)", ""),
            // ("topk(some_metric,)", ""),
            // ("topk(some_metric, other_metric)", ""),
            // ("count_values(5, other_metric)", ""),
            // ("rate(some_metric[5m]) @ 1234", ""),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    #[ignore]
    fn test_function_call_parser() {
        // "time()"
        // floor(some_metric{foo!="bar"})
        // "rate(some_metric[5m])"
        // "round(some_metric)"
        // "round(some_metric, 5)"

        let fail_cases = vec![
            // ("floor()", ""),
            // ("floor(some_metric, other_metric)", ""),
            // ("floor(some_metric, 1)", ""),
            // ("floor(1)", ""),
            // ("hour(some_metric, some_metric, some_metric)", ""),
            // ("time(some_metric)", ""),
            // ("non_existent_function_far_bar()", ""),
            // ("rate(some_metric)", ""),
            // (r#"label_replace(a, `b`, `c\xff`, `d`, `.*`)"#, ""),

        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    #[ignore]
    fn test_subquery_parser() {}

    #[test]
    #[ignore]
    fn test_preprocessors() {}

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

            // Fuzzing regression tests.
            // ("-=", r#"unexpected "=""#),
            // ("++-++-+-+-<", "unexpected <op:<>"),
            // ("e-+=/(0)", r#"unexpected "=""#),
            // ("a>b()", "unknown function"),
            // ("rate(avg)", "expected type range vector"),

            // "(" + strings.Repeat("-{}-1", 10000) + ")" + strings.Repeat("[1m:]", 1000)
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }
}
