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

use promql_parser::parser;
use promql_parser::parser::function::Function;
use promql_parser::parser::value::ValueType;

#[test]
fn test_parse_with_custom_function() {
    // Register custom functions (like Thanos xrate/xdelta/xincrease)
    parser::register_extra_functions(vec![
        Function::new("xrate", vec![ValueType::Matrix], 0, ValueType::Vector, true),
        Function::new(
            "xdelta",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            true,
        ),
        Function::new(
            "xincrease",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            true,
        ),
    ])
    .unwrap();

    // Custom functions should parse successfully
    let result = parser::parse("xrate(http_requests_total[5m])");
    assert!(result.is_ok(), "xrate should parse: {:?}", result.err());

    let result = parser::parse("xdelta(temperature[1h])");
    assert!(result.is_ok(), "xdelta should parse: {:?}", result.err());

    let result = parser::parse("xincrease(counter[10m])");
    assert!(result.is_ok(), "xincrease should parse: {:?}", result.err());

    // Custom functions work in nested expressions
    let result = parser::parse("sum(xrate(http_requests_total[5m])) by (instance)");
    assert!(
        result.is_ok(),
        "nested xrate should parse: {:?}",
        result.err()
    );

    // Built-in functions still work
    let result = parser::parse("rate(http_requests_total[5m])");
    assert!(result.is_ok(), "built-in rate should still work");

    // Unknown functions still fail
    let result = parser::parse("totally_unknown_func(up)");
    assert!(result.is_err(), "unknown function should still fail");
}

#[test]
fn test_custom_function_arg_validation() {
    // Register a function that expects a matrix arg
    parser::register_extra_functions(vec![Function::new(
        "custom_over_time",
        vec![ValueType::Matrix],
        0,
        ValueType::Vector,
        true,
    )])
    .unwrap();

    // Correct usage (matrix selector)
    let result = parser::parse("custom_over_time(metric[5m])");
    assert!(
        result.is_ok(),
        "correct args should parse: {:?}",
        result.err()
    );

    // Wrong arg type (vector instead of matrix) should fail
    let result = parser::parse("custom_over_time(metric)");
    assert!(result.is_err(), "vector arg for matrix param should fail");
}
