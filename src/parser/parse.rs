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

use crate::parser::{lex, Expr, INVALID_QUERY_INFO};

/// Parse the given query literal to an AST (which is [`Expr`] in this crate).
pub fn parse(input: &str) -> Result<Expr, String> {
    match lex::lexer(input) {
        Err(e) => Err(e),
        Ok(lexer) => {
            // NOTE: the errs is ignored so far.
            let (res, _errs) = crate::promql_y::parse(&lexer);
            res.ok_or_else(|| String::from(INVALID_QUERY_INFO))?
        }
    }
}

/// cases in original prometheus is a huge slices which are constructed more than 3000 lines,
/// and it is hard to split them based on the original order. So here is the Note:
///
/// - all cases SHOULD be covered, and the same literal float and literal
///   string SHOULD be the same with the original prometheus.
/// - all cases will be split into different blocks based on the type of parsed Expr.
#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::label::{Labels, MatchOp, Matcher, Matchers, METRIC_NAME};
    use crate::parser;
    use crate::parser::function::get_function;
    use crate::parser::{
        token, AtModifier as At, BinModifier, Expr, FunctionArgs, LabelModifier, Offset,
        VectorMatchCardinality, VectorSelector, INVALID_QUERY_INFO,
    };
    use crate::util::duration;
    use std::time::Duration;
    use std::vec;

    struct Case {
        input: String,
        expected: Result<Expr, String>,
    }

    impl Case {
        fn new(input: &str, expected: Result<Expr, String>) -> Self {
            Case {
                input: String::from(input),
                expected,
            }
        }

        fn new_result_cases(cases: Vec<(&str, Result<Expr, String>)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, expected)| Case::new(input, expected))
                .collect()
        }

        fn new_expr_cases(cases: Vec<(&str, Expr)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, expected)| Case::new(input, Ok(expected)))
                .collect()
        }

        fn new_fail_cases(cases: Vec<(&str, &str)>) -> Vec<Case> {
            cases
                .into_iter()
                .map(|(input, expected)| Case::new(input, Err(expected.into())))
                .collect()
        }
    }

    fn assert_cases(cases: Vec<Case>) {
        for Case { input, expected } in cases {
            assert_eq!(expected, crate::parser::parse(&input));
        }
    }

    #[test]
    fn test_number_literal() {
        let cases = vec![
            ("1", Expr::from(1.0)),
            ("Inf", Expr::from(f64::INFINITY)),
            ("+Inf", Expr::from(f64::INFINITY)),
            ("-Inf", Expr::from(f64::NEG_INFINITY)),
            (".5", Expr::from(0.5)),
            ("5.", Expr::from(5.0)),
            ("123.4567", Expr::from(123.4567)),
            ("5e-3", Expr::from(0.005)),
            ("5e3", Expr::from(5000.0)),
            ("0xc", Expr::from(12.0)),
            ("0755", Expr::from(493.0)),
            ("08", Expr::from(8.0)),
            ("+5.5e-3", Expr::from(0.0055)),
            ("-0755", Expr::from(-493.0)),

            // for abnormal input
            ("NaN", Expr::from(f64::NAN)),
            (
                "999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999",
                Expr::from(f64::INFINITY)
            ),
        ];
        assert_cases(Case::new_expr_cases(cases));
    }

    #[test]
    fn test_string_literal() {
        let cases = vec![
            (
                "\"double-quoted string \\\" with escaped quote\"",
                Expr::from("double-quoted string \\\" with escaped quote"),
            ),
            (
                // this case is the same with the previous upper one
                r#""double-quoted string \" with escaped quote""#,
                Expr::from(r#"double-quoted string \" with escaped quote"#),
            ),
            (
                r"'single-quoted string \' with escaped quote'",
                Expr::from(r"single-quoted string \' with escaped quote"),
            ),
            (
                "`backtick-quoted string`",
                Expr::from("backtick-quoted string"),
            ),
            (
                r#""\a\b\f\n\r\t\v\\\" - \xFF\377\u1234\U00010111\U0001011111☺""#,
                Expr::from(r#"\a\b\f\n\r\t\v\\\" - \xFF\377\u1234\U00010111\U0001011111☺"#),
            ),
            (
                r"'\a\b\f\n\r\t\v\\\' - \xFF\377\u1234\U00010111\U0001011111☺'",
                Expr::from(r"\a\b\f\n\r\t\v\\\' - \xFF\377\u1234\U00010111\U0001011111☺"),
            ),
            (
                r"`\a\b\f\n\r\t\v\\\` - \xFF\377\u1234\U00010111\U0001011111☺`",
                Expr::from(r"\a\b\f\n\r\t\v\\\` - \xFF\377\u1234\U00010111\U0001011111☺"),
            ),
        ];
        assert_cases(Case::new_expr_cases(cases));

        let fail_cases = vec![
            (r"`\\``", "unterminated quoted string `"),
            (r#""\"#, "escape sequence not terminated"),
            (r#""\c""#, "unknown escape sequence 'c'"),
            // (r#""\x.""#, ""),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    fn test_vector_binary_expr() {
        let cases = vec![
            (
                "1 + 1",
                Expr::new_binary_expr(Expr::from(1.0), token::T_ADD, None, Expr::from(1.0)),
            ),
            (
                "1 - 1",
                Expr::new_binary_expr(Expr::from(1.0), token::T_SUB, None, Expr::from(1.0)),
            ),
            (
                "1 * 1",
                Expr::new_binary_expr(Expr::from(1.0), token::T_MUL, None, Expr::from(1.0)),
            ),
            (
                "1 / 1",
                Expr::new_binary_expr(Expr::from(1.0), token::T_DIV, None, Expr::from(1.0)),
            ),
            (
                "1 % 1",
                Expr::new_binary_expr(Expr::from(1.0), token::T_MOD, None, Expr::from(1.0)),
            ),
            (
                "1 == bool 1",
                Expr::new_binary_expr(
                    Expr::from(1.0),
                    token::T_EQLC,
                    Some(BinModifier::default().with_return_bool(true)),
                    Expr::from(1.0),
                ),
            ),
            (
                "1 != bool 1",
                Expr::new_binary_expr(
                    Expr::from(1.0),
                    token::T_NEQ,
                    Some(BinModifier::default().with_return_bool(true)),
                    Expr::from(1.0),
                ),
            ),
            (
                "1 > bool 1",
                Expr::new_binary_expr(
                    Expr::from(1.0),
                    token::T_GTR,
                    Some(BinModifier::default().with_return_bool(true)),
                    Expr::from(1.0),
                ),
            ),
            (
                "1 >= bool 1",
                Expr::new_binary_expr(
                    Expr::from(1.0),
                    token::T_GTE,
                    Some(BinModifier::default().with_return_bool(true)),
                    Expr::from(1.0),
                ),
            ),
            (
                "1 < bool 1",
                Expr::new_binary_expr(
                    Expr::from(1.0),
                    token::T_LSS,
                    Some(BinModifier::default().with_return_bool(true)),
                    Expr::from(1.0),
                ),
            ),
            (
                "1 <= bool 1",
                Expr::new_binary_expr(
                    Expr::from(1.0),
                    token::T_LTE,
                    Some(BinModifier::default().with_return_bool(true)),
                    Expr::from(1.0),
                ),
            ),
            (
                "-1^2",
                Expr::new_binary_expr(Expr::from(1.0), token::T_POW, None, Expr::from(2.0))
                    .map(|ex| -ex),
            ),
            (
                "-1*2",
                Expr::new_binary_expr(Expr::from(-1.0), token::T_MUL, None, Expr::from(2.0)),
            ),
            (
                "-1+2",
                Expr::new_binary_expr(Expr::from(-1.0), token::T_ADD, None, Expr::from(2.0)),
            ),
            (
                "-1^-2",
                Expr::new_binary_expr(Expr::from(1.0), token::T_POW, None, Expr::from(-2.0))
                    .map(|ex| -ex),
            ),
            (
                "+1 + -2 * 1",
                Expr::new_binary_expr(Expr::from(-2.0), token::T_MUL, None, Expr::from(1.0))
                    .and_then(|ex| Expr::new_binary_expr(Expr::from(1.0), token::T_ADD, None, ex)),
            ),
            (
                "1 + 2/(3*1)",
                Expr::new_binary_expr(Expr::from(3.0), token::T_MUL, None, Expr::from(1.0))
                    .and_then(Expr::new_paren_expr)
                    .and_then(|ex| Expr::new_binary_expr(Expr::from(2.0), token::T_DIV, None, ex))
                    .and_then(|ex| Expr::new_binary_expr(Expr::from(1.0), token::T_ADD, None, ex)),
            ),
            (
                "1 < bool 2 - 1 * 2",
                Expr::new_binary_expr(Expr::from(1.0), token::T_MUL, None, Expr::from(2.0))
                    .and_then(|ex| Expr::new_binary_expr(Expr::from(2.0), token::T_SUB, None, ex))
                    .and_then(|ex| {
                        Expr::new_binary_expr(
                            Expr::from(1.0),
                            token::T_LSS,
                            Some(BinModifier::default().with_return_bool(true)),
                            ex,
                        )
                    }),
            ),
            (
                "foo * bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_MUL,
                    None,
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo * sum",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_MUL,
                    None,
                    Expr::from(VectorSelector::from("sum")),
                ),
            ),
            (
                "foo == 1",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_EQLC,
                    None,
                    Expr::from(1.0),
                ),
            ),
            (
                "foo == bool 1",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_EQLC,
                    Some(BinModifier::default().with_return_bool(true)),
                    Expr::from(1.0),
                ),
            ),
            (
                "2.5 / bar",
                Expr::new_binary_expr(
                    Expr::from(2.5),
                    token::T_DIV,
                    None,
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo and bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LAND,
                    Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany)),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo or bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LOR,
                    Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany)),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo unless bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LUNLESS,
                    Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany)),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                // Test and/or precedence and reassigning of operands.
                "foo + bar or bla and blub",
                {
                    let lhs = Expr::new_binary_expr(
                        Expr::from(VectorSelector::from("foo")),
                        token::T_ADD,
                        None,
                        Expr::from(VectorSelector::from("bar")),
                    );
                    let rhs = Expr::new_binary_expr(
                        Expr::from(VectorSelector::from("bla")),
                        token::T_LAND,
                        Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany)),
                        Expr::from(VectorSelector::from("blub")),
                    );
                    Expr::new_binary_expr(
                        lhs.unwrap(),
                        token::T_LOR,
                        Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany)),
                        rhs.unwrap(),
                    )
                },
            ),
            (
                // Test and/or/unless precedence.
                "foo and bar unless baz or qux",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LAND,
                    Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany)),
                    Expr::from(VectorSelector::from("bar")),
                )
                .and_then(|ex| {
                    Expr::new_binary_expr(
                        ex,
                        token::T_LUNLESS,
                        Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany)),
                        Expr::from(VectorSelector::from("baz")),
                    )
                })
                .and_then(|ex| {
                    Expr::new_binary_expr(
                        ex,
                        token::T_LOR,
                        Some(BinModifier::default().with_card(VectorMatchCardinality::ManyToMany)),
                        Expr::from(VectorSelector::from("qux")),
                    )
                }),
            ),
            (
                // Test precedence and reassigning of operands.
                "bar + on(foo) bla / on(baz, buz) group_right(test) blub",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("bla")),
                    token::T_DIV,
                    Some(
                        BinModifier::default()
                            .with_card(VectorMatchCardinality::one_to_many(vec!["test"]))
                            .with_matching(Some(LabelModifier::include(vec!["baz", "buz"]))),
                    ),
                    Expr::from(VectorSelector::from("blub")),
                )
                .and_then(|ex| {
                    Expr::new_binary_expr(
                        Expr::from(VectorSelector::from("bar")),
                        token::T_ADD,
                        Some(
                            BinModifier::default()
                                .with_matching(Some(LabelModifier::include(vec!["foo"]))),
                        ),
                        ex,
                    )
                }),
            ),
            (
                "foo * on(test,blub) bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_MUL,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(LabelModifier::include(vec!["test", "blub"]))),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo * on(test,blub) group_left bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_MUL,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(LabelModifier::include(vec!["test", "blub"])))
                            .with_card(VectorMatchCardinality::many_to_one(vec![])),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            ("foo and on(test,blub) bar", {
                let matching = LabelModifier::include(vec!["test", "blub"]);
                let card = VectorMatchCardinality::ManyToMany;
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LAND,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(matching))
                            .with_card(card),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                )
            }),
            ("foo and on() bar", {
                let matching = LabelModifier::include(vec![]);
                let card = VectorMatchCardinality::ManyToMany;
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LAND,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(matching))
                            .with_card(card),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                )
            }),
            ("foo and ignoring(test,blub) bar", {
                let matching = LabelModifier::exclude(vec!["test", "blub"]);
                let card = VectorMatchCardinality::ManyToMany;
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LAND,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(matching))
                            .with_card(card),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                )
            }),
            ("foo and ignoring() bar", {
                let matching = LabelModifier::exclude(vec![]);
                let card = VectorMatchCardinality::ManyToMany;
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LAND,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(matching))
                            .with_card(card),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                )
            }),
            ("foo unless on(bar) baz", {
                let matching = LabelModifier::include(vec!["bar"]);
                let card = VectorMatchCardinality::ManyToMany;
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LUNLESS,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(matching))
                            .with_card(card),
                    ),
                    Expr::from(VectorSelector::from("baz")),
                )
            }),
            (
                "foo / on(test,blub) group_left(bar) bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_DIV,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(LabelModifier::include(vec!["test", "blub"])))
                            .with_card(VectorMatchCardinality::many_to_one(vec!["bar"])),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo / ignoring(test,blub) group_left(blub) bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_DIV,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(LabelModifier::exclude(vec!["test", "blub"])))
                            .with_card(VectorMatchCardinality::many_to_one(vec!["blub"])),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo / ignoring(test,blub) group_left(bar) bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_DIV,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(LabelModifier::exclude(vec!["test", "blub"])))
                            .with_card(VectorMatchCardinality::many_to_one(vec!["bar"])),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo - on(test,blub) group_right(bar,foo) bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_SUB,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(LabelModifier::include(vec!["test", "blub"])))
                            .with_card(VectorMatchCardinality::one_to_many(vec!["bar", "foo"])),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "foo - ignoring(test,blub) group_right(bar,foo) bar",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_SUB,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(LabelModifier::exclude(vec!["test", "blub"])))
                            .with_card(VectorMatchCardinality::one_to_many(vec!["bar", "foo"])),
                    ),
                    Expr::from(VectorSelector::from("bar")),
                ),
            ),
            (
                "a + sum",
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("a")),
                    token::T_ADD,
                    None,
                    Expr::from(VectorSelector::from("sum")),
                ),
            ),
            // cases from https://prometheus.io/docs/prometheus/latest/querying/operators
            (
                r#"method_code:http_errors:rate5m{code="500"} / ignoring(code) method:http_requests:rate5m"#,
                {
                    let name = String::from("method_code:http_errors:rate5m");
                    let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "code", "500"));
                    let lhs = Expr::new_vector_selector(Some(name), matchers).unwrap();
                    Expr::new_binary_expr(
                        lhs,
                        token::T_DIV,
                        Some(
                            BinModifier::default()
                                .with_matching(Some(LabelModifier::exclude(vec!["code"]))),
                        ),
                        Expr::from(VectorSelector::from("method:http_requests:rate5m")),
                    )
                },
            ),
            (
                r#"method_code:http_errors:rate5m / ignoring(code) group_left method:http_requests:rate5m"#,
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("method_code:http_errors:rate5m")),
                    token::T_DIV,
                    Some(
                        BinModifier::default()
                            .with_matching(Some(LabelModifier::exclude(vec!["code"])))
                            .with_card(VectorMatchCardinality::ManyToOne(Labels::new(vec![]))),
                    ),
                    Expr::from(VectorSelector::from("method:http_requests:rate5m")),
                ),
            ),
        ];
        assert_cases(Case::new_result_cases(cases));

        let fail_cases = vec![
            (
                "foo and 1",
                "set operator 'and' not allowed in binary scalar expression",
            ),
            (
                "1 and foo",
                "set operator 'and' not allowed in binary scalar expression",
            ),
            (
                "foo or 1",
                "set operator 'or' not allowed in binary scalar expression",
            ),
            (
                "1 or foo",
                "set operator 'or' not allowed in binary scalar expression",
            ),
            (
                "foo unless 1",
                "set operator 'unless' not allowed in binary scalar expression",
            ),
            (
                "1 unless foo",
                "set operator 'unless' not allowed in binary scalar expression",
            ),
            (
                "1 or on(bar) foo",
                "set operator 'or' not allowed in binary scalar expression",
            ),
            (
                "foo == on(bar) 10",
                "vector matching only allowed between vectors",
            ),
            // NOTE: group modifier CAN NOT be used without on/ignoring modifier
            ("foo + group_left(baz) bar", "unexpected <group_left>"),
            (
                "foo and on(bar) group_left(baz) bar",
                "no grouping allowed for 'and' operation",
            ),
            (
                "foo and on(bar) group_right(baz) bar",
                "no grouping allowed for 'and' operation",
            ),
            (
                "foo or on(bar) group_left(baz) bar",
                "no grouping allowed for 'or' operation",
            ),
            (
                "foo or on(bar) group_right(baz) bar",
                "no grouping allowed for 'or' operation",
            ),
            (
                "foo unless on(bar) group_left(baz) bar",
                "no grouping allowed for 'unless' operation",
            ),
            (
                "foo unless on(bar) group_right(baz) bar",
                "no grouping allowed for 'unless' operation",
            ),
            (
                r#"http_requests{group="production"} + on(instance) group_left(job,instance) cpu_count{type="smp"}"#,
                "label 'instance' must not occur in ON and GROUP clause at once",
            ),
            (
                "foo + bool bar",
                "bool modifier can only be used on comparison operators",
            ),
            (
                "foo + bool 10",
                "bool modifier can only be used on comparison operators",
            ),
            (
                "foo and bool 10",
                "bool modifier can only be used on comparison operators",
            ),
            (
                "1 and 1",
                "set operator 'and' not allowed in binary scalar expression",
            ),
            (
                "1 == 1",
                "comparisons between scalars must use BOOL modifier",
            ),
            (
                "1 or 1",
                "set operator 'or' not allowed in binary scalar expression",
            ),
            (
                "1 unless 1",
                "set operator 'unless' not allowed in binary scalar expression",
            ),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    fn test_unary_expr() {
        let cases = vec![
            (
                "-some_metric",
                Expr::new_unary_expr(Expr::from(VectorSelector::from("some_metric"))).unwrap(),
            ),
            (
                "+some_metric",
                Expr::from(VectorSelector::from("some_metric")),
            ),
            (
                " +some_metric",
                Expr::from(VectorSelector::from("some_metric")),
            ),
        ];
        assert_cases(Case::new_expr_cases(cases));

        let cases = vec![
            (r#"-"string""#, "unary expression only allowed on expressions of type scalar or vector, got: string"),
            ("-test[5m]", "unary expression only allowed on expressions of type scalar or vector, got: matrix"),
            (r#"-"foo""#, "unary expression only allowed on expressions of type scalar or vector, got: string"),
        ];
        assert_cases(Case::new_fail_cases(cases));
    }

    #[test]
    fn test_vector_selector() {
        let cases = vec![
            ("foo", Ok(Expr::from(VectorSelector::from("foo")))),
            ("min", Ok(Expr::from(VectorSelector::from("min")))),
            (
                "foo offset 5m",
                Expr::from(VectorSelector::from("foo"))
                    .offset_expr(Offset::Pos(Duration::from_secs(60 * 5))),
            ),
            (
                "foo offset -7m",
                Expr::from(VectorSelector::from("foo"))
                    .offset_expr(Offset::Neg(Duration::from_secs(60 * 7))),
            ),
            (
                "foo OFFSET 1h30m",
                Expr::from(VectorSelector::from("foo"))
                    .offset_expr(Offset::Pos(Duration::from_secs(60 * 90))),
            ),
            (
                "foo OFFSET 1h30ms",
                Expr::from(VectorSelector::from("foo")).offset_expr(Offset::Pos(
                    Duration::from_secs(60 * 60) + Duration::from_millis(30),
                )),
            ),
            (
                "foo @ 1603774568",
                Expr::from(VectorSelector::from("foo"))
                    .at_expr(At::try_from(1603774568f64).unwrap()),
            ),
            (
                "foo @ -100",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(-100f64).unwrap()),
            ),
            (
                "foo @ .3",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(0.3f64).unwrap()),
            ),
            (
                "foo @ 3.",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(3.0f64).unwrap()),
            ),
            (
                "foo @ 3.33",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(3.33f64).unwrap()),
            ),
            (
                "foo @ 3.3333",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(3.333f64).unwrap()),
            ),
            (
                "foo @ 3.3335",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(3.334f64).unwrap()),
            ),
            (
                "foo @ 3e2",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(300f64).unwrap()),
            ),
            (
                "foo @ 3e-1",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(0.3f64).unwrap()),
            ),
            (
                "foo @ 0xA",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(10f64).unwrap()),
            ),
            (
                "foo @ -3.3e1",
                Expr::from(VectorSelector::from("foo")).at_expr(At::try_from(-33f64).unwrap()),
            ),
            (r#"foo:bar{a="bc"}"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "a", "bc"));
                Expr::new_vector_selector(Some(String::from("foo:bar")), matchers)
            }),
            (r#"foo{NaN='bc'}"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "NaN", "bc"));
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
            }),
            (r#"foo{bar='}'}"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "bar", "}"));
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
            }),
            (r#"foo{a="b", foo!="bar", test=~"test", bar!~"baz"}"#, {
                let matchers = Matchers::new(vec![
                    Matcher::new(MatchOp::Equal, "a", "b"),
                    Matcher::new(MatchOp::NotEqual, "foo", "bar"),
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
                ]);
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
            }),
            (r#"foo{a="b", foo!="bar", test=~"test", bar!~"baz",}"#, {
                let name = String::from("foo");
                let matchers = Matchers::new(vec![
                    Matcher::new(MatchOp::Equal, "a", "b"),
                    Matcher::new(MatchOp::NotEqual, "foo", "bar"),
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
                ]);
                Expr::new_vector_selector(Some(name), matchers)
            }),
            // the following multiple __name__ matcher test cases are not from prometheus
            (r#"{__name__="foo",__name__="bar"}"#, {
                let matchers = Matchers::new(vec![
                    Matcher::new(MatchOp::Equal, METRIC_NAME, "foo"),
                    Matcher::new(MatchOp::Equal, METRIC_NAME, "bar"),
                ]);
                Expr::new_vector_selector(None, matchers)
            }),
            (r#"{__name__=~"foo.+",__name__=~".*bar"}"#, {
                let matchers = Matchers::new(vec![
                    Matcher::new_matcher(
                        token::T_EQL_REGEX,
                        String::from(METRIC_NAME),
                        String::from("foo.+"),
                    )
                    .unwrap(),
                    Matcher::new_matcher(
                        token::T_EQL_REGEX,
                        String::from(METRIC_NAME),
                        String::from(".*bar"),
                    )
                    .unwrap(),
                ]);
                Expr::new_vector_selector(None, matchers)
            }),
            (r#"foo:bar{a=~"bc{9}"}"#, {
                let matchers = Matchers::one(Matcher::new(
                    MatchOp::Re(Regex::new("bc{9}").unwrap()),
                    "a",
                    "bc{9}",
                ));
                Expr::new_vector_selector(Some(String::from("foo:bar")), matchers)
            }),
            (r#"foo:bar{a=~"bc{abc}"}"#, {
                let matchers = Matchers::one(Matcher::new(
                    MatchOp::Re(Regex::new("bc\\{abc}").unwrap()),
                    "a",
                    "bc{abc}",
                ));
                Expr::new_vector_selector(Some(String::from("foo:bar")), matchers)
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
            (
                "some_metric{a=b}",
                "unexpected identifier 'b' in label matching, expected string",
            ),
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
            (
                "foo{gibberish}",
                "invalid label matcher, expected label matching operator after 'gibberish'",
            ),
            ("foo{1}", "unexpected character inside braces: '1'"),
            (
                "{}",
                "vector selector must contain at least one non-empty matcher",
            ),
            (
                r#"{x=""}"#,
                "vector selector must contain at least one non-empty matcher",
            ),
            (
                r#"{x=~".*"}"#,
                "vector selector must contain at least one non-empty matcher",
            ),
            (
                r#"{x!~".+"}"#,
                "vector selector must contain at least one non-empty matcher",
            ),
            (
                r#"{x!="a"}"#,
                "vector selector must contain at least one non-empty matcher",
            ),
            (
                r#"foo{__name__="bar"}"#,
                "metric name must not be set twice: 'foo' or 'bar'",
            ),
            (
                "foo{__name__= =}",
                "unexpected '=' in label matching, expected string",
            ),
            (
                "foo{,}",
                r#"unexpected ',' in label matching, expected identifier or right_brace"#,
            ),
            (
                r#"foo{__name__ == "bar"}"#,
                "unexpected '=' in label matching, expected string",
            ),
            (
                r#"foo{__name__="bar" lol}"#,
                // "invalid label matcher, expected label matching operator after 'lol'",
                INVALID_QUERY_INFO,
            ),
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
            (
                "test[5s]",
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("test")),
                    Duration::from_secs(5),
                ),
            ),
            (
                "test[5m]",
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("test")),
                    duration::MINUTE_DURATION * 5,
                ),
            ),
            (
                "test[5m30s]",
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("test")),
                    Duration::from_secs(330),
                ),
            ),
            (
                "test[5h] OFFSET 5m",
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("test")),
                    duration::HOUR_DURATION * 5,
                )
                .and_then(|ex| ex.offset_expr(Offset::Pos(duration::MINUTE_DURATION * 5))),
            ),
            (
                "test[5d] OFFSET 10s",
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("test")),
                    duration::DAY_DURATION * 5,
                )
                .and_then(|ex| ex.offset_expr(Offset::Pos(Duration::from_secs(10)))),
            ),
            (
                "test[5w] offset 2w",
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("test")),
                    duration::WEEK_DURATION * 5,
                )
                .and_then(|ex| ex.offset_expr(Offset::Pos(duration::WEEK_DURATION * 2))),
            ),
            (r#"test{a="b"}[5y] OFFSET 3d"#, {
                Expr::new_vector_selector(
                    Some(String::from("test")),
                    Matchers::one(Matcher::new(MatchOp::Equal, "a", "b")),
                )
                .and_then(|ex| Expr::new_matrix_selector(ex, duration::YEAR_DURATION * 5))
                .and_then(|ex| ex.offset_expr(Offset::Pos(duration::DAY_DURATION * 3)))
            }),
            (r#"test{a="b"}[5y] @ 1603774699"#, {
                Expr::new_vector_selector(
                    Some(String::from("test")),
                    Matchers::one(Matcher::new(MatchOp::Equal, "a", "b")),
                )
                .and_then(|ex| Expr::new_matrix_selector(ex, duration::YEAR_DURATION * 5))
                .and_then(|ex| ex.at_expr(At::try_from(1603774699_f64).unwrap()))
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
            (
                r#"foo["5m"]"#,
                r#"unexpected character inside brackets: '"'"#,
            ),
            (r#"foo[]"#, "missing unit character in duration"),
            (r#"foo[1]"#, r#"bad duration syntax: 1]"#),
            (
                "some_metric[5m] OFFSET 1",
                "unexpected number '1' in offset, expected duration",
            ),
            (
                "some_metric[5m] OFFSET 1mm",
                "bad number or duration syntax: 1mm",
            ),
            (
                "some_metric[5m] OFFSET",
                "unexpected end of input in offset, expected duration",
            ),
            (
                "some_metric OFFSET 1m[5m]",
                "no offset modifiers allowed before range",
            ),
            (
                "some_metric[5m] @ 1m",
                "unexpected duration '1m' in @, expected timestamp",
            ),
            (
                "some_metric[5m] @",
                "unexpected end of input in @, expected timestamp",
            ),
            (
                "some_metric @ 1234 [5m]",
                "no @ modifiers allowed before range",
            ),
            (
                "(foo + bar)[5m]",
                "ranges only allowed for vector selectors",
            ),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    fn test_aggregation_expr() {
        let cases = vec![
            ("sum by (foo) (some_metric)", {
                let ex = Expr::from(VectorSelector::from("some_metric"));
                let modifier = LabelModifier::include(vec!["foo"]);
                Expr::new_aggregate_expr(token::T_SUM, Some(modifier), FunctionArgs::new_args(ex))
            }),
            ("avg by (foo)(some_metric)", {
                let ex = Expr::from(VectorSelector::from("some_metric"));
                let modifier = LabelModifier::include(vec!["foo"]);
                Expr::new_aggregate_expr(token::T_AVG, Some(modifier), FunctionArgs::new_args(ex))
            }),
            ("max by (foo)(some_metric)", {
                let modifier = LabelModifier::include(vec!["foo"]);
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_aggregate_expr(token::T_MAX, Some(modifier), FunctionArgs::new_args(ex))
            }),
            ("sum without (foo) (some_metric)", {
                let modifier = LabelModifier::exclude(vec!["foo"]);
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_aggregate_expr(token::T_SUM, Some(modifier), FunctionArgs::new_args(ex))
            }),
            ("sum (some_metric) without (foo)", {
                let modifier = LabelModifier::exclude(vec!["foo"]);
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_aggregate_expr(token::T_SUM, Some(modifier), FunctionArgs::new_args(ex))
            }),
            ("stddev(some_metric)", {
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_aggregate_expr(token::T_STDDEV, None, FunctionArgs::new_args(ex))
            }),
            ("stdvar by (foo)(some_metric)", {
                let modifier = LabelModifier::include(vec!["foo"]);
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_aggregate_expr(
                    token::T_STDVAR,
                    Some(modifier),
                    FunctionArgs::new_args(ex),
                )
            }),
            ("sum by ()(some_metric)", {
                let modifier = LabelModifier::include(vec![]);
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_aggregate_expr(token::T_SUM, Some(modifier), FunctionArgs::new_args(ex))
            }),
            ("sum by (foo,bar,)(some_metric)", {
                let modifier = LabelModifier::include(vec!["foo", "bar"]);
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_aggregate_expr(token::T_SUM, Some(modifier), FunctionArgs::new_args(ex))
            }),
            ("sum by (foo,)(some_metric)", {
                let modifier = LabelModifier::include(vec!["foo"]);
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_aggregate_expr(token::T_SUM, Some(modifier), FunctionArgs::new_args(ex))
            }),
            ("topk(5, some_metric)", {
                let ex = Expr::from(VectorSelector::from("some_metric"));
                let param = Expr::from(5.0);
                let args = FunctionArgs::new_args(param).append_args(ex);
                Expr::new_aggregate_expr(token::T_TOPK, None, args)
            }),
            (r#"count_values("value", some_metric)"#, {
                let ex = Expr::from(VectorSelector::from("some_metric"));
                let param = Expr::from("value");
                let args = FunctionArgs::new_args(param).append_args(ex);
                Expr::new_aggregate_expr(token::T_COUNT_VALUES, None, args)
            }),
            (
                "sum without(and, by, avg, count, alert, annotations)(some_metric)",
                {
                    let modifier = LabelModifier::exclude(vec![
                        "and",
                        "by",
                        "avg",
                        "count",
                        "alert",
                        "annotations",
                    ]);
                    let ex = Expr::from(VectorSelector::from("some_metric"));
                    Expr::new_aggregate_expr(
                        token::T_SUM,
                        Some(modifier),
                        FunctionArgs::new_args(ex),
                    )
                },
            ),
            ("sum(sum)", {
                let ex = Expr::from(VectorSelector::from("sum"));
                Expr::new_aggregate_expr(token::T_SUM, None, FunctionArgs::new_args(ex))
            }),
        ];
        assert_cases(Case::new_result_cases(cases));

        let fail_cases = vec![
            ("sum without(==)(some_metric)", INVALID_QUERY_INFO),
            ("sum without(,)(some_metric)", INVALID_QUERY_INFO),
            ("sum without(foo,,)(some_metric)", INVALID_QUERY_INFO),
            ("sum some_metric by (test)", INVALID_QUERY_INFO),
            ("sum (some_metric) by test", INVALID_QUERY_INFO),
            (
                "sum () by (test)",
                "no arguments for aggregate expression 'sum' provided",
            ),
            ("MIN keep_common (some_metric)", INVALID_QUERY_INFO),
            ("MIN (some_metric) keep_common", INVALID_QUERY_INFO),
            ("sum (some_metric) without (test) by (test)", INVALID_QUERY_INFO),
            ("sum without (test) (some_metric) by (test)", INVALID_QUERY_INFO),
            (
                "topk(some_metric)",
                "wrong number of arguments for aggregate expression provided, expected 2, got 1",
            ),
            (
                "topk(some_metric,)",
                "trailing commas not allowed in function call args",
            ),
            (
                "topk(some_metric, other_metric)",
                "expected type scalar in aggregation expression, got vector",
            ),
            (
                "count_values(5, other_metric)",
                "expected type string in aggregation expression, got scalar",
            ),
            (
                "rate(some_metric[5m]) @ 1234",
                "@ modifier must be preceded by an vector selector or matrix selector or a subquery"
            ),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    fn test_function_call() {
        let cases = vec![
            (
                "time()",
                Expr::new_call(get_function("time").unwrap(), FunctionArgs::empty_args()),
            ),
            (r#"floor(some_metric{foo!="bar"})"#, {
                let name = String::from("some_metric");
                let matchers = Matchers::one(Matcher::new(MatchOp::NotEqual, "foo", "bar"));
                let ex = Expr::new_vector_selector(Some(name), matchers).unwrap();
                Expr::new_call(get_function("floor").unwrap(), FunctionArgs::new_args(ex))
            }),
            ("rate(some_metric[5m])", {
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("some_metric")),
                    duration::MINUTE_DURATION * 5,
                )
                .and_then(|ex| {
                    Expr::new_call(get_function("rate").unwrap(), FunctionArgs::new_args(ex))
                })
            }),
            ("round(some_metric)", {
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_call(get_function("round").unwrap(), FunctionArgs::new_args(ex))
            }),
            ("round(some_metric, 5)", {
                let ex = Expr::from(VectorSelector::from("some_metric"));
                Expr::new_call(
                    get_function("round").unwrap(),
                    FunctionArgs::new_args(ex).append_args(Expr::from(5.0)),
                )
            }),
            // cases from https://prometheus.io/docs/prometheus/latest/querying/functions
            (r#"absent(nonexistent{job="myjob"})"#, {
                let name = String::from("nonexistent");
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "job", "myjob"));
                let ex = Expr::new_vector_selector(Some(name), matchers).unwrap();
                Expr::new_call(get_function("absent").unwrap(), FunctionArgs::new_args(ex))
            }),
            (r#"absent(nonexistent{job="myjob",instance=~".*"})"#, {
                let name = String::from("nonexistent");
                let matchers = Matchers::new(vec![
                    Matcher::new(MatchOp::Equal, "job", "myjob"),
                    Matcher::new(MatchOp::Re(Regex::new(".*").unwrap()), "instance", ".*"),
                ]);
                Expr::new_vector_selector(Some(name), matchers).and_then(|ex| {
                    Expr::new_call(get_function("absent").unwrap(), FunctionArgs::new_args(ex))
                })
            }),
            (r#"absent(sum(nonexistent{job="myjob"}))"#, {
                let name = String::from("nonexistent");
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "job", "myjob"));
                Expr::new_vector_selector(Some(name), matchers)
                    .and_then(|ex| {
                        Expr::new_aggregate_expr(token::T_SUM, None, FunctionArgs::new_args(ex))
                    })
                    .and_then(|ex| {
                        Expr::new_call(get_function("absent").unwrap(), FunctionArgs::new_args(ex))
                    })
            }),
            (r#"absent_over_time(nonexistent{job="myjob"}[1h])"#, {
                let name = String::from("nonexistent");
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "job", "myjob"));
                Expr::new_vector_selector(Some(name), matchers)
                    .and_then(|ex| Expr::new_matrix_selector(ex, duration::HOUR_DURATION))
                    .and_then(|ex| {
                        Expr::new_call(
                            get_function("absent_over_time").unwrap(),
                            FunctionArgs::new_args(ex),
                        )
                    })
            }),
            (
                r#"absent_over_time(nonexistent{job="myjob",instance=~".*"}[1h])"#,
                {
                    let name = String::from("nonexistent");
                    let matchers = Matchers::new(vec![
                        Matcher::new(MatchOp::Equal, "job", "myjob"),
                        Matcher::new(MatchOp::Re(Regex::new(".*").unwrap()), "instance", ".*"),
                    ]);
                    Expr::new_vector_selector(Some(name), matchers)
                        .and_then(|ex| Expr::new_matrix_selector(ex, duration::HOUR_DURATION))
                        .and_then(|ex| {
                            Expr::new_call(
                                get_function("absent_over_time").unwrap(),
                                FunctionArgs::new_args(ex),
                            )
                        })
                },
            ),
            (r#"delta(cpu_temp_celsius{host="zeus"}[2h])"#, {
                let name = String::from("cpu_temp_celsius");
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "host", "zeus"));
                Expr::new_vector_selector(Some(name), matchers)
                    .and_then(|ex| Expr::new_matrix_selector(ex, duration::HOUR_DURATION * 2))
                    .and_then(|ex| {
                        Expr::new_call(get_function("delta").unwrap(), FunctionArgs::new_args(ex))
                    })
            }),
            (
                r#"histogram_count(rate(http_request_duration_seconds[10m]))"#,
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("http_request_duration_seconds")),
                    duration::MINUTE_DURATION * 10,
                )
                .and_then(|ex| {
                    Expr::new_call(get_function("rate").unwrap(), FunctionArgs::new_args(ex))
                })
                .and_then(|ex| {
                    Expr::new_call(
                        get_function("histogram_count").unwrap(),
                        FunctionArgs::new_args(ex),
                    )
                }),
            ),
            (
                r#"histogram_sum(rate(http_request_duration_seconds[10m])) / histogram_count(rate(http_request_duration_seconds[10m]))"#,
                {
                    let rate = Expr::new_matrix_selector(
                        Expr::from(VectorSelector::from("http_request_duration_seconds")),
                        duration::MINUTE_DURATION * 10,
                    )
                    .and_then(|ex| {
                        Expr::new_call(get_function("rate").unwrap(), FunctionArgs::new_args(ex))
                    })
                    .unwrap();
                    let lhs = Expr::new_call(
                        get_function("histogram_sum").unwrap(),
                        FunctionArgs::new_args(rate.clone()),
                    )
                    .unwrap();
                    let rhs = Expr::new_call(
                        get_function("histogram_count").unwrap(),
                        FunctionArgs::new_args(rate),
                    )
                    .unwrap();
                    Expr::new_binary_expr(lhs, token::T_DIV, None, rhs)
                },
            ),
            (
                r#"histogram_fraction(0, 0.2, rate(http_request_duration_seconds[1h]))"#,
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("http_request_duration_seconds")),
                    duration::HOUR_DURATION,
                )
                .and_then(|ex| {
                    Expr::new_call(get_function("rate").unwrap(), FunctionArgs::new_args(ex))
                })
                .and_then(|ex| {
                    Expr::new_call(
                        get_function("histogram_fraction").unwrap(),
                        FunctionArgs::new_args(Expr::from(0.0_f64))
                            .append_args(Expr::from(0.2))
                            .append_args(ex),
                    )
                }),
            ),
            (
                r#"histogram_quantile(0.9, rate(http_request_duration_seconds_bucket[10m]))"#,
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("http_request_duration_seconds_bucket")),
                    duration::MINUTE_DURATION * 10,
                )
                .and_then(|ex| {
                    Expr::new_call(get_function("rate").unwrap(), FunctionArgs::new_args(ex))
                })
                .and_then(|ex| {
                    Expr::new_call(
                        get_function("histogram_quantile").unwrap(),
                        FunctionArgs::new_args(Expr::from(0.9_f64)).append_args(ex),
                    )
                }),
            ),
            (
                r#"histogram_quantile(0.9, sum by (job, le) (rate(http_request_duration_seconds_bucket[10m])))"#,
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("http_request_duration_seconds_bucket")),
                    duration::MINUTE_DURATION * 10,
                )
                .and_then(|ex| {
                    Expr::new_call(get_function("rate").unwrap(), FunctionArgs::new_args(ex))
                })
                .and_then(|ex| {
                    Expr::new_aggregate_expr(
                        token::T_SUM,
                        Some(LabelModifier::include(vec!["job", "le"])),
                        FunctionArgs::new_args(ex),
                    )
                })
                .and_then(|ex| {
                    Expr::new_call(
                        get_function("histogram_quantile").unwrap(),
                        FunctionArgs::new_args(Expr::from(0.9_f64)).append_args(ex),
                    )
                }),
            ),
            (r#"increase(http_requests_total{job="api-server"}[5m])"#, {
                let name = String::from("http_requests_total");
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "job", "api-server"));
                Expr::new_vector_selector(Some(name), matchers)
                    .and_then(|ex| Expr::new_matrix_selector(ex, duration::MINUTE_DURATION * 5))
                    .and_then(|ex| {
                        Expr::new_call(
                            get_function("increase").unwrap(),
                            FunctionArgs::new_args(ex),
                        )
                    })
            }),
            (r#"irate(http_requests_total{job="api-server"}[5m])"#, {
                let name = String::from("http_requests_total");
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "job", "api-server"));
                Expr::new_vector_selector(Some(name), matchers)
                    .and_then(|ex| Expr::new_matrix_selector(ex, duration::MINUTE_DURATION * 5))
                    .and_then(|ex| {
                        Expr::new_call(get_function("irate").unwrap(), FunctionArgs::new_args(ex))
                    })
            }),
            (
                r#"label_join(up{job="api-server",src1="a",src2="b",src3="c"}, "foo", ",", "src1", "src2", "src3")"#,
                {
                    let name = String::from("up");
                    let matchers = Matchers::new(vec![
                        Matcher::new(MatchOp::Equal, "job", "api-server"),
                        Matcher::new(MatchOp::Equal, "src1", "a"),
                        Matcher::new(MatchOp::Equal, "src2", "b"),
                        Matcher::new(MatchOp::Equal, "src3", "c"),
                    ]);
                    Expr::new_vector_selector(Some(name), matchers).and_then(|ex| {
                        Expr::new_call(
                            get_function("label_join").unwrap(),
                            FunctionArgs::new_args(ex)
                                .append_args(Expr::from("foo"))
                                .append_args(Expr::from(","))
                                .append_args(Expr::from("src1"))
                                .append_args(Expr::from("src2"))
                                .append_args(Expr::from("src3")),
                        )
                    })
                },
            ),
            (
                r#"label_replace(up{job="api-server",service="a:c"}, "foo", "$1", "service", "(.*):.*")"#,
                {
                    let name = String::from("up");
                    let matchers = Matchers::new(vec![
                        Matcher::new(MatchOp::Equal, "job", "api-server"),
                        Matcher::new(MatchOp::Equal, "service", "a:c"),
                    ]);
                    Expr::new_vector_selector(Some(name), matchers).and_then(|ex| {
                        Expr::new_call(
                            get_function("label_replace").unwrap(),
                            FunctionArgs::new_args(ex)
                                .append_args(Expr::from("foo"))
                                .append_args(Expr::from("$1"))
                                .append_args(Expr::from("service"))
                                .append_args(Expr::from("(.*):.*")),
                        )
                    })
                },
            ),
            // special cases
            (
                r#"exp(+Inf)"#,
                Expr::new_call(
                    get_function("exp").unwrap(),
                    FunctionArgs::new_args(Expr::from(f64::INFINITY)),
                ),
            ),
            (
                r#"exp(NaN)"#,
                Expr::new_call(
                    get_function("exp").unwrap(),
                    FunctionArgs::new_args(Expr::from(f64::NAN)),
                ),
            ),
            (
                r#"ln(+Inf)"#,
                Expr::new_call(
                    get_function("ln").unwrap(),
                    FunctionArgs::new_args(Expr::from(f64::INFINITY)),
                ),
            ),
            (
                r#"ln(NaN)"#,
                Expr::new_call(
                    get_function("ln").unwrap(),
                    FunctionArgs::new_args(Expr::from(f64::NAN)),
                ),
            ),
            (
                r#"ln(0)"#,
                Expr::new_call(
                    get_function("ln").unwrap(),
                    FunctionArgs::new_args(Expr::from(0.0)),
                ),
            ),
            (
                r#"ln(-1)"#,
                Expr::new_call(
                    get_function("ln").unwrap(),
                    FunctionArgs::new_args(Expr::from(-1.0)),
                ),
            ),
            (
                r#"log2(+Inf)"#,
                Expr::new_call(
                    get_function("log2").unwrap(),
                    FunctionArgs::new_args(Expr::from(f64::INFINITY)),
                ),
            ),
            (
                r#"log2(NaN)"#,
                Expr::new_call(
                    get_function("log2").unwrap(),
                    FunctionArgs::new_args(Expr::from(f64::NAN)),
                ),
            ),
            (
                r#"log2(0)"#,
                Expr::new_call(
                    get_function("log2").unwrap(),
                    FunctionArgs::new_args(Expr::from(0.0)),
                ),
            ),
            (
                r#"log2(-1)"#,
                Expr::new_call(
                    get_function("log2").unwrap(),
                    FunctionArgs::new_args(Expr::from(-1.0)),
                ),
            ),
            (
                r#"log10(+Inf)"#,
                Expr::new_call(
                    get_function("log10").unwrap(),
                    FunctionArgs::new_args(Expr::from(f64::INFINITY)),
                ),
            ),
            (
                r#"log10(NaN)"#,
                Expr::new_call(
                    get_function("log10").unwrap(),
                    FunctionArgs::new_args(Expr::from(f64::NAN)),
                ),
            ),
            (
                r#"log10(0)"#,
                Expr::new_call(
                    get_function("log10").unwrap(),
                    FunctionArgs::new_args(Expr::from(0.0)),
                ),
            ),
            (
                r#"log10(-1)"#,
                Expr::new_call(
                    get_function("log10").unwrap(),
                    FunctionArgs::new_args(Expr::from(-1.0)),
                ),
            ),
        ];

        assert_cases(Case::new_result_cases(cases));

        let fail_cases = vec![
            (
                "floor()",
                "expected 1 argument(s) in call to 'floor', got 0",
            ),
            (
                "floor(some_metric, other_metric)",
                "expected 1 argument(s) in call to 'floor', got 2",
            ),
            (
                "floor(some_metric, 1)",
                "expected 1 argument(s) in call to 'floor', got 2",
            ),
            (
                "floor(1)",
                "expected type vector in call to function 'floor', got scalar",
            ),
            (
                "hour(some_metric, some_metric, some_metric)",
                "expected at most 1 argument(s) in call to 'hour', got 3",
            ),
            (
                "time(some_metric)",
                "expected 0 argument(s) in call to 'time', got 1",
            ),
            (
                "non_existent_function_far_bar()",
                "unknown function with name 'non_existent_function_far_bar'",
            ),
            (
                "rate(some_metric)",
                "expected type matrix in call to function 'rate', got vector",
            ),
            (
                "ln(1)",
                "expected type vector in call to function 'ln', got scalar",
            ),
            ("ln()", "expected 1 argument(s) in call to 'ln', got 0"),
            (
                "exp(1)",
                "expected type vector in call to function 'exp', got scalar",
            ),
            ("exp()", "expected 1 argument(s) in call to 'exp', got 0"),
            (
                "label_join()",
                "expected at least 3 argument(s) in call to 'label_join', got 0",
            ),
            // (r#"label_replace(a, `b`, `c\xff`, `d`, `.*`)"#, ""),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    fn test_subquery() {
        let cases = vec![
            (r#"foo{bar="baz"}[10m:6s]"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "bar", "baz"));
                Expr::new_vector_selector(Some(String::from("foo")), matchers).and_then(|ex| {
                    Expr::new_subquery_expr(
                        ex,
                        duration::MINUTE_DURATION * 10,
                        Some(duration::SECOND_DURATION * 6),
                    )
                })
            }),
            (r#"foo{bar="baz"}[10m5s:1h6ms]"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "bar", "baz"));
                Expr::new_vector_selector(Some(String::from("foo")), matchers).and_then(|ex| {
                    Expr::new_subquery_expr(
                        ex,
                        duration::MINUTE_DURATION * 10 + duration::SECOND_DURATION * 5,
                        Some(duration::HOUR_DURATION + duration::MILLI_DURATION * 6),
                    )
                })
            }),
            ("foo[10m:]", {
                let ex = Expr::from(VectorSelector::from("foo"));
                Expr::new_subquery_expr(ex, duration::MINUTE_DURATION * 10, None)
            }),
            (r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:5s])"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "bar", "baz"));
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
                    .and_then(|ex| Expr::new_matrix_selector(ex, Duration::from_secs(2)))
                    .and_then(|ex| {
                        Expr::new_call(get_function("rate").unwrap(), FunctionArgs::new_args(ex))
                    })
                    .and_then(|ex| {
                        Expr::new_subquery_expr(
                            ex,
                            duration::MINUTE_DURATION * 5,
                            Some(Duration::from_secs(5)),
                        )
                    })
                    .and_then(|ex| {
                        Expr::new_call(
                            get_function("min_over_time").unwrap(),
                            FunctionArgs::new_args(ex),
                        )
                    })
            }),
            (r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:])[4m:3s]"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "bar", "baz"));
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
                    .and_then(|ex| Expr::new_matrix_selector(ex, Duration::from_secs(2)))
                    .and_then(|ex| {
                        Expr::new_call(get_function("rate").unwrap(), FunctionArgs::new_args(ex))
                    })
                    .and_then(|ex| Expr::new_subquery_expr(ex, duration::MINUTE_DURATION * 5, None))
                    .and_then(|ex| {
                        Expr::new_call(
                            get_function("min_over_time").unwrap(),
                            FunctionArgs::new_args(ex),
                        )
                    })
                    .and_then(|ex| {
                        Expr::new_subquery_expr(
                            ex,
                            duration::MINUTE_DURATION * 4,
                            Some(Duration::from_secs(3)),
                        )
                    })
            }),
            (
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:] offset 4m)[4m:3s]"#,
                {
                    let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "bar", "baz"));
                    Expr::new_vector_selector(Some(String::from("foo")), matchers)
                        .and_then(|ex| Expr::new_matrix_selector(ex, Duration::from_secs(2)))
                        .and_then(|ex| {
                            Expr::new_call(
                                get_function("rate").unwrap(),
                                FunctionArgs::new_args(ex),
                            )
                        })
                        .and_then(|ex| {
                            Expr::new_subquery_expr(ex, duration::MINUTE_DURATION * 5, None)
                        })
                        .and_then(|ex| ex.offset_expr(Offset::Pos(duration::MINUTE_DURATION * 4)))
                        .and_then(|ex| {
                            Expr::new_call(
                                get_function("min_over_time").unwrap(),
                                FunctionArgs::new_args(ex),
                            )
                        })
                        .and_then(|ex| {
                            Expr::new_subquery_expr(
                                ex,
                                duration::MINUTE_DURATION * 4,
                                Some(Duration::from_secs(3)),
                            )
                        })
                },
            ),
            (
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:] @ 1603775091)[4m:3s]"#,
                {
                    let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "bar", "baz"));
                    Expr::new_vector_selector(Some(String::from("foo")), matchers)
                        .and_then(|ex| Expr::new_matrix_selector(ex, Duration::from_secs(2)))
                        .and_then(|ex| {
                            Expr::new_call(
                                get_function("rate").unwrap(),
                                FunctionArgs::new_args(ex),
                            )
                        })
                        .and_then(|ex| {
                            Expr::new_subquery_expr(ex, duration::MINUTE_DURATION * 5, None)
                        })
                        .and_then(|ex| ex.at_expr(At::try_from(1603775091_f64).unwrap()))
                        .and_then(|ex| {
                            Expr::new_call(
                                get_function("min_over_time").unwrap(),
                                FunctionArgs::new_args(ex),
                            )
                        })
                        .and_then(|ex| {
                            Expr::new_subquery_expr(
                                ex,
                                duration::MINUTE_DURATION * 4,
                                Some(Duration::from_secs(3)),
                            )
                        })
                },
            ),
            (
                r#"min_over_time(rate(foo{bar="baz"}[2s])[5m:] @ -160377509)[4m:3s]"#,
                {
                    let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "bar", "baz"));
                    Expr::new_vector_selector(Some(String::from("foo")), matchers)
                        .and_then(|ex| Expr::new_matrix_selector(ex, Duration::from_secs(2)))
                        .and_then(|ex| {
                            Expr::new_call(
                                get_function("rate").unwrap(),
                                FunctionArgs::new_args(ex),
                            )
                        })
                        .and_then(|ex| {
                            Expr::new_subquery_expr(ex, duration::MINUTE_DURATION * 5, None)
                        })
                        .and_then(|ex| ex.at_expr(At::try_from(-160377509_f64).unwrap()))
                        .and_then(|ex| {
                            Expr::new_call(
                                get_function("min_over_time").unwrap(),
                                FunctionArgs::new_args(ex),
                            )
                        })
                        .and_then(|ex| {
                            Expr::new_subquery_expr(
                                ex,
                                duration::MINUTE_DURATION * 4,
                                Some(Duration::from_secs(3)),
                            )
                        })
                },
            ),
            (
                "sum without(and, by, avg, count, alert, annotations)(some_metric) [30m:10s]",
                {
                    let ex = Expr::from(VectorSelector::from("some_metric"));
                    Expr::new_aggregate_expr(
                        token::T_SUM,
                        Some(LabelModifier::exclude(vec![
                            "and",
                            "by",
                            "avg",
                            "count",
                            "alert",
                            "annotations",
                        ])),
                        FunctionArgs::new_args(ex),
                    )
                    .and_then(|ex| {
                        Expr::new_subquery_expr(
                            ex,
                            duration::MINUTE_DURATION * 30,
                            Some(Duration::from_secs(10)),
                        )
                    })
                },
            ),
            (
                "some_metric OFFSET 1m [10m:5s]",
                Expr::from(VectorSelector::from("some_metric"))
                    .offset_expr(Offset::Pos(duration::MINUTE_DURATION))
                    .and_then(|ex| {
                        Expr::new_subquery_expr(
                            ex,
                            duration::MINUTE_DURATION * 10,
                            Some(Duration::from_secs(5)),
                        )
                    }),
            ),
            (
                "some_metric @ 123 [10m:5s]",
                Expr::from(VectorSelector::from("some_metric"))
                    .at_expr(At::try_from(123_f64).unwrap())
                    .and_then(|ex| {
                        Expr::new_subquery_expr(
                            ex,
                            duration::MINUTE_DURATION * 10,
                            Some(Duration::from_secs(5)),
                        )
                    }),
            ),
            (
                "some_metric @ 123 offset 1m [10m:5s]",
                Expr::from(VectorSelector::from("some_metric"))
                    .at_expr(At::try_from(123_f64).unwrap())
                    .and_then(|ex| ex.offset_expr(Offset::Pos(duration::MINUTE_DURATION)))
                    .and_then(|ex| {
                        Expr::new_subquery_expr(
                            ex,
                            duration::MINUTE_DURATION * 10,
                            Some(Duration::from_secs(5)),
                        )
                    }),
            ),
            (
                "some_metric offset 1m @ 123 [10m:5s]",
                Expr::from(VectorSelector::from("some_metric"))
                    .at_expr(At::try_from(123_f64).unwrap())
                    .and_then(|ex| ex.offset_expr(Offset::Pos(duration::MINUTE_DURATION)))
                    .and_then(|ex| {
                        Expr::new_subquery_expr(
                            ex,
                            duration::MINUTE_DURATION * 10,
                            Some(Duration::from_secs(5)),
                        )
                    }),
            ),
            (
                "some_metric[10m:5s] offset 1m @ 123",
                Expr::new_subquery_expr(
                    Expr::from(VectorSelector::from("some_metric")),
                    duration::MINUTE_DURATION * 10,
                    Some(Duration::from_secs(5)),
                )
                .and_then(|ex| ex.at_expr(At::try_from(123_f64).unwrap()))
                .and_then(|ex| ex.offset_expr(Offset::Pos(duration::MINUTE_DURATION))),
            ),
            (r#"(foo + bar{nm="val"})[5m:]"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "nm", "val"));
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_ADD,
                    None,
                    Expr::new_vector_selector(Some(String::from("bar")), matchers).unwrap(),
                )
                .and_then(Expr::new_paren_expr)
                .and_then(|ex| Expr::new_subquery_expr(ex, duration::MINUTE_DURATION * 5, None))
            }),
            (r#"(foo + bar{nm="val"})[5m:] offset 10m"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "nm", "val"));
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_ADD,
                    None,
                    Expr::new_vector_selector(Some(String::from("bar")), matchers).unwrap(),
                )
                .and_then(Expr::new_paren_expr)
                .and_then(|ex| Expr::new_subquery_expr(ex, duration::MINUTE_DURATION * 5, None))
                .and_then(|ex| ex.offset_expr(Offset::Pos(duration::MINUTE_DURATION * 10)))
            }),
            (r#"(foo + bar{nm="val"} @ 1234)[5m:] @ 1603775019"#, {
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "nm", "val"));
                let rhs = Expr::new_vector_selector(Some(String::from("bar")), matchers)
                    .and_then(|ex| ex.at_expr(At::try_from(1234_f64).unwrap()))
                    .unwrap();

                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_ADD,
                    None,
                    rhs,
                )
                .and_then(Expr::new_paren_expr)
                .and_then(|ex| Expr::new_subquery_expr(ex, duration::MINUTE_DURATION * 5, None))
                .and_then(|ex| ex.at_expr(At::try_from(1603775019_f64).unwrap()))
            }),
        ];
        assert_cases(Case::new_result_cases(cases));

        let fail_cases = vec![
            (
                "test[5d] OFFSET 10s [10m:5s]",
                "subquery is only allowed on vector, got matrix instead",
            ),
            (
                r#"(foo + bar{nm="val"})[5m:][10m:5s]"#,
                "subquery is only allowed on vector, got matrix instead",
            ),
            (
                "rate(food[1m])[1h] offset 1h",
                "ranges only allowed for vector selectors",
            ),
            (
                "rate(food[1m])[1h] @ 100",
                "ranges only allowed for vector selectors",
            ),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }

    #[test]
    fn test_preprocessors() {
        let cases = vec![
            (
                "foo @ start()",
                Expr::from(VectorSelector::from("foo")).at_expr(At::Start),
            ),
            (
                "foo @ end()",
                Expr::from(VectorSelector::from("foo")).at_expr(At::End),
            ),
            (
                "test[5y] @ start()",
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("test")),
                    duration::YEAR_DURATION * 5,
                )
                .and_then(|ex| ex.at_expr(At::Start)),
            ),
            (
                "test[5y] @ end()",
                Expr::new_matrix_selector(
                    Expr::from(VectorSelector::from("test")),
                    duration::YEAR_DURATION * 5,
                )
                .and_then(|ex| ex.at_expr(At::End)),
            ),
            (
                "foo[10m:6s] @ start()",
                Expr::new_subquery_expr(
                    Expr::from(VectorSelector::from("foo")),
                    duration::MINUTE_DURATION * 10,
                    Some(Duration::from_secs(6)),
                )
                .and_then(|ex| ex.at_expr(At::Start)),
            ),
            // Check that start and end functions do not mask metrics.
            ("start", Ok(Expr::from(VectorSelector::from("start")))),
            ("end", Ok(Expr::from(VectorSelector::from("end")))),
            (r#"start{end="foo"}"#, {
                let name = String::from("start");
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "end", "foo"));
                Expr::new_vector_selector(Some(name), matchers)
            }),
            (r#"end{start="foo"}"#, {
                let name = String::from("end");
                let matchers = Matchers::one(Matcher::new(MatchOp::Equal, "start", "foo"));
                Expr::new_vector_selector(Some(name), matchers)
            }),
            ("foo unless on(start) bar", {
                let modifier = BinModifier::default()
                    .with_matching(Some(LabelModifier::include(vec!["start"])))
                    .with_card(VectorMatchCardinality::ManyToMany);
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LUNLESS,
                    Some(modifier),
                    Expr::from(VectorSelector::from("bar")),
                )
            }),
            ("foo unless on(end) bar", {
                let modifier = BinModifier::default()
                    .with_matching(Some(LabelModifier::include(vec!["end"])))
                    .with_card(VectorMatchCardinality::ManyToMany);
                Expr::new_binary_expr(
                    Expr::from(VectorSelector::from("foo")),
                    token::T_LUNLESS,
                    Some(modifier),
                    Expr::from(VectorSelector::from("bar")),
                )
            }),
        ];
        assert_cases(Case::new_result_cases(cases));

        let cases = vec![
            ("start()", INVALID_QUERY_INFO),
            ("end()", INVALID_QUERY_INFO),
        ];
        assert_cases(Case::new_fail_cases(cases));
    }

    #[test]
    fn test_corner_fail_cases() {
        let fail_cases = vec![
            ("", "no expression found in input"),
            (
                "# just a comment\n\n",
                "no expression found in input",
            ),
            ("1+", INVALID_QUERY_INFO),
            (".", "unexpected character: '.'"),
            ("2.5.", "bad number or duration syntax: 2.5."),
            ("100..4", "bad number or duration syntax: 100.."),
            ("0deadbeef", "bad number or duration syntax: 0de"),
            ("1 /", INVALID_QUERY_INFO),
            ("*1", INVALID_QUERY_INFO),
            ("(1))", "unexpected right parenthesis ')'"),
            ("((1)", "unclosed left parenthesis"),
            ("(", "unclosed left parenthesis"),
            ("1 !~ 1", "unexpected character after '!': '~'"),
            ("1 =~ 1", "unexpected character after '=': '~'"),
            ("*test", INVALID_QUERY_INFO),
            (
                "1 offset 1d",
                "offset modifier must be preceded by an vector selector or matrix selector or a subquery"
            ),
            (
                "foo offset 1s offset 2s",
                "offset may not be set multiple times"
            ),
            ("a - on(b) ignoring(c) d", INVALID_QUERY_INFO),

            // Fuzzing regression tests.
            ("-=", INVALID_QUERY_INFO),
            ("++-++-+-+-<", INVALID_QUERY_INFO),
            ("e-+=/(0)", INVALID_QUERY_INFO),
            ("a>b()", "unknown function with name 'b'"),
            (
                "rate(avg)",
                "expected type matrix in call to function 'rate', got vector"
            ),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));

        let fail_cases = vec![
            // This is testing that we are not re-rendering the expression string for each error, which would timeout.
            {
                let input = "(".to_string() + &"-{}-1".repeat(10_000) + ")" + &"[1m:]".repeat(1000);
                let expected =
                    Err("vector selector must contain at least one non-empty matcher".into());
                Case { input, expected }
            },
        ];
        assert_cases(fail_cases);
    }

    #[test]
    fn test_or_filters() {
        let cases = vec![
            (r#"foo{label1="1" or label1="2"}"#, {
                let matchers = Matchers::new(vec![]).with_or_matchers(vec![
                    vec![Matcher::new(MatchOp::Equal, "label1", "1")],
                    vec![Matcher::new(MatchOp::Equal, "label1", "2")],
                ]);
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
            }),
            (r#"foo{label1="1" OR label1="2"}"#, {
                let matchers = Matchers::new(vec![]).with_or_matchers(vec![
                    vec![Matcher::new(MatchOp::Equal, "label1", "1")],
                    vec![Matcher::new(MatchOp::Equal, "label1", "2")],
                ]);
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
            }),
            (r#"foo{label1="1" Or label1="2"}"#, {
                let matchers = Matchers::new(vec![]).with_or_matchers(vec![
                    vec![Matcher::new(MatchOp::Equal, "label1", "1")],
                    vec![Matcher::new(MatchOp::Equal, "label1", "2")],
                ]);
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
            }),
            (r#"foo{label1="1" oR label1="2"}"#, {
                let matchers = Matchers::new(vec![]).with_or_matchers(vec![
                    vec![Matcher::new(MatchOp::Equal, "label1", "1")],
                    vec![Matcher::new(MatchOp::Equal, "label1", "2")],
                ]);
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
            }),
            (r#"foo{label1="1" or or="or"}"#, {
                let matchers = Matchers::new(vec![]).with_or_matchers(vec![
                    vec![Matcher::new(MatchOp::Equal, "label1", "1")],
                    vec![Matcher::new(MatchOp::Equal, "or", "or")],
                ]);
                Expr::new_vector_selector(Some(String::from("foo")), matchers)
            }),
            (
                r#"foo{label1="1" or label1="2" or label1="3" or label1="4"}"#,
                {
                    let matchers = Matchers::new(vec![]).with_or_matchers(vec![
                        vec![Matcher::new(MatchOp::Equal, "label1", "1")],
                        vec![Matcher::new(MatchOp::Equal, "label1", "2")],
                        vec![Matcher::new(MatchOp::Equal, "label1", "3")],
                        vec![Matcher::new(MatchOp::Equal, "label1", "4")],
                    ]);
                    Expr::new_vector_selector(Some(String::from("foo")), matchers)
                },
            ),
            (
                r#"foo{label1="1" or label1="2" or label1="3", label2="4"}"#,
                {
                    let matchers = Matchers::new(vec![]).with_or_matchers(vec![
                        vec![Matcher::new(MatchOp::Equal, "label1", "1")],
                        vec![Matcher::new(MatchOp::Equal, "label1", "2")],
                        vec![
                            Matcher::new(MatchOp::Equal, "label1", "3"),
                            Matcher::new(MatchOp::Equal, "label2", "4"),
                        ],
                    ]);
                    Expr::new_vector_selector(Some(String::from("foo")), matchers)
                },
            ),
            (
                r#"foo{label1="1", label2="2" or label1="3" or label1="4"}"#,
                {
                    let matchers = Matchers::new(vec![]).with_or_matchers(vec![
                        vec![
                            Matcher::new(MatchOp::Equal, "label1", "1"),
                            Matcher::new(MatchOp::Equal, "label2", "2"),
                        ],
                        vec![Matcher::new(MatchOp::Equal, "label1", "3")],
                        vec![Matcher::new(MatchOp::Equal, "label1", "4")],
                    ]);
                    Expr::new_vector_selector(Some(String::from("foo")), matchers)
                },
            ),
        ];
        assert_cases(Case::new_result_cases(cases));

        let display_cases = vec![
            r#"a{label1="1"}"#,
            r#"a{label1="1" or label2="2"}"#,
            r#"a{label1="1" or label2="2" or label3="3" or label4="4"}"#,
            r#"a{label1="1", label2="2" or label3="3" or label4="4"}"#,
            r#"a{label1="1", label2="2" or label3="3", label4="4"}"#,
        ];
        display_cases
            .iter()
            .for_each(|expr| assert_eq!(parser::parse(expr).unwrap().to_string(), *expr));

        let fail_cases = vec![
            (
                r#"foo{or}"#,
                r#"invalid label matcher, expected label matching operator after 'or'"#,
            ),
            (r#"foo{label1="1" or}"#, INVALID_QUERY_INFO),
            (r#"foo{or label1="1"}"#, INVALID_QUERY_INFO),
            (r#"foo{label1="1" or or label2="2"}"#, INVALID_QUERY_INFO),
        ];
        assert_cases(Case::new_fail_cases(fail_cases));
    }
}
