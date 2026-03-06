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

use std::collections::HashMap;
use std::fmt;

use lazy_static::lazy_static;

use crate::parser::value::ValueType;
use crate::parser::{Expr, Prettier};
use crate::util::join_vector;

/// called by func in Call
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ser", derive(serde::Serialize))]
pub struct FunctionArgs {
    pub args: Vec<Box<Expr>>,
}

impl FunctionArgs {
    pub fn empty_args() -> Self {
        Self { args: vec![] }
    }

    pub fn new_args(expr: Expr) -> Self {
        Self {
            args: vec![Box::new(expr)],
        }
    }

    pub fn append_args(mut self: FunctionArgs, expr: Expr) -> Self {
        self.args.push(Box::new(expr));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn first(&self) -> Option<Box<Expr>> {
        self.args.first().cloned()
    }

    pub fn last(&self) -> Option<Box<Expr>> {
        self.args.last().cloned()
    }
}

impl fmt::Display for FunctionArgs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", join_vector(&self.args, ", ", false))
    }
}

impl Prettier for FunctionArgs {
    fn pretty(&self, level: usize, max: usize) -> String {
        let mut v = vec![];
        for ex in &self.args {
            v.push(ex.pretty(level, max));
        }
        v.join(",\n")
    }
}

/// Functions is a list of all functions supported by PromQL, including their types.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ser", derive(serde::Serialize))]
#[cfg_attr(feature = "ser", serde(rename_all = "camelCase"))]
pub struct Function {
    pub name: &'static str,
    pub arg_types: Vec<ValueType>,
    /// Variadic cardinality follows Prometheus semantics:
    /// 0 = exact args, >0 = bounded optional args, <0 = unbounded args.
    pub variadic: i32,
    pub return_type: ValueType,
    pub experimental: bool,
}

impl Function {
    pub fn new(
        name: &'static str,
        arg_types: Vec<ValueType>,
        variadic: i32,
        return_type: ValueType,
        experimental: bool,
    ) -> Self {
        Self {
            name,
            arg_types,
            variadic,
            return_type,
            experimental,
        }
    }
}

macro_rules! function {
    ($name:expr, $arg_types:expr, $variadic:expr, $return_type:expr, $experimental:expr) => {
        (
            $name,
            Function::new($name, $arg_types, $variadic, $return_type, $experimental),
        )
    };
}

lazy_static! {
    static ref FUNCTIONS: HashMap<&'static str, Function> = HashMap::from([
        function!("abs", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "absent",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "absent_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!("acos", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "acosh",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!("asin", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "asinh",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!("atan", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "atanh",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "avg_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!("ceil", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "changes",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "clamp",
            vec![ValueType::Vector, ValueType::Scalar, ValueType::Scalar],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "clamp_max",
            vec![ValueType::Vector, ValueType::Scalar],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "clamp_min",
            vec![ValueType::Vector, ValueType::Scalar],
            0,
            ValueType::Vector,
            false
        ),
        function!("cos", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!("cosh", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "count_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "days_in_month",
            vec![ValueType::Vector],
            1,
            ValueType::Vector,
            false
        ),
        function!(
            "day_of_month",
            vec![ValueType::Vector],
            1,
            ValueType::Vector,
            false
        ),
        function!(
            "day_of_week",
            vec![ValueType::Vector],
            1,
            ValueType::Vector,
            false
        ),
        function!(
            "day_of_year",
            vec![ValueType::Vector],
            1,
            ValueType::Vector,
            false
        ),
        function!("deg", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "delta",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "deriv",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!("exp", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "floor",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "histogram_count",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "histogram_sum",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "histogram_avg",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "histogram_fraction",
            vec![ValueType::Scalar, ValueType::Scalar, ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "histogram_quantile",
            vec![ValueType::Scalar, ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "histogram_stddev",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "histogram_stdvar",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "double_exponential_smoothing",
            vec![ValueType::Matrix, ValueType::Scalar, ValueType::Scalar],
            0,
            ValueType::Vector,
            true
        ),
        function!(
            "holt_winters",
            vec![ValueType::Matrix, ValueType::Scalar, ValueType::Scalar],
            0,
            ValueType::Vector,
            false
        ),
        function!("hour", vec![ValueType::Vector], 1, ValueType::Vector, false),
        function!(
            "idelta",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "increase",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "irate",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "label_replace",
            vec![
                ValueType::Vector,
                ValueType::String,
                ValueType::String,
                ValueType::String,
                ValueType::String
            ],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "label_join",
            vec![
                ValueType::Vector,
                ValueType::String,
                ValueType::String,
                ValueType::String
            ],
            -1,
            ValueType::Vector,
            false
        ),
        function!(
            "last_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!("ln", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "log10",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!("log2", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "max_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "min_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "minute",
            vec![ValueType::Vector],
            1,
            ValueType::Vector,
            false
        ),
        function!(
            "month",
            vec![ValueType::Vector],
            1,
            ValueType::Vector,
            false
        ),
        function!("pi", vec![], 0, ValueType::Scalar, false),
        function!(
            "predict_linear",
            vec![ValueType::Matrix, ValueType::Scalar],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "present_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "quantile_over_time",
            vec![ValueType::Scalar, ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!("rad", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!("rate", vec![ValueType::Matrix], 0, ValueType::Vector, false),
        function!(
            "resets",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "round",
            vec![ValueType::Vector, ValueType::Scalar],
            1,
            ValueType::Vector,
            false
        ),
        function!(
            "scalar",
            vec![ValueType::Vector],
            0,
            ValueType::Scalar,
            false
        ),
        function!("sgn", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!("sin", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!("sinh", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!("sort", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "sort_desc",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "sort_by_label",
            vec![ValueType::Vector, ValueType::String, ValueType::String],
            -1,
            ValueType::Vector,
            true
        ),
        function!(
            "sort_by_label_desc",
            vec![ValueType::Vector, ValueType::String, ValueType::String],
            -1,
            ValueType::Vector,
            true
        ),
        function!("sqrt", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!(
            "stddev_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "stdvar_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "sum_over_time",
            vec![ValueType::Matrix],
            0,
            ValueType::Vector,
            false
        ),
        function!("tan", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!("tanh", vec![ValueType::Vector], 0, ValueType::Vector, false),
        function!("time", vec![], 0, ValueType::Scalar, false),
        function!(
            "timestamp",
            vec![ValueType::Vector],
            0,
            ValueType::Vector,
            false
        ),
        function!(
            "vector",
            vec![ValueType::Scalar],
            0,
            ValueType::Vector,
            false
        ),
        function!("year", vec![ValueType::Vector], 1, ValueType::Vector, false),
    ]);
}

/// get_function returns a predefined Function object for the given name.
pub(crate) fn get_function(name: &str) -> Option<Function> {
    FUNCTIONS.get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::*;

    #[test]
    fn test_function_equality() {
        let func = "month";
        assert!(get_function(func).is_some());
        assert_eq!(get_function(func), get_function(func));
    }

    #[test]
    fn test_function_args_equality() {
        assert_eq!(FunctionArgs::empty_args(), FunctionArgs::empty_args());

        let arg1 = Expr::NumberLiteral(NumberLiteral::new(1.0));
        let arg2 = Expr::StringLiteral(StringLiteral {
            val: "prometheus".into(),
        });
        let args1 = FunctionArgs::new_args(arg1).append_args(arg2);

        let arg1 = Expr::NumberLiteral(NumberLiteral::new(0.5 + 0.5));
        let arg2 = Expr::StringLiteral(StringLiteral {
            val: String::from("prometheus"),
        });
        let args2 = FunctionArgs::new_args(arg1).append_args(arg2);

        assert_eq!(args1, args2);
    }

    #[test]
    fn test_args_display() {
        let cases = vec![
            (
                FunctionArgs::new_args(Expr::from(VectorSelector::from("up"))),
                "up",
            ),
            (
                FunctionArgs::empty_args()
                    .append_args(Expr::from("src1"))
                    .append_args(Expr::from("src2"))
                    .append_args(Expr::from("src3")),
                r#""src1", "src2", "src3""#,
            ),
        ];

        for (args, expect) in cases {
            assert_eq!(expect, args.to_string())
        }
    }

    #[test]
    fn test_function_metadata() {
        let round = get_function("round").unwrap();
        assert_eq!(round.variadic, 1);
        assert!(!round.experimental);

        let label_join = get_function("label_join").unwrap();
        assert_eq!(label_join.variadic, -1);
        assert!(!label_join.experimental);

        let sort_by_label = get_function("sort_by_label").unwrap();
        assert_eq!(sort_by_label.variadic, -1);
        assert!(sort_by_label.experimental);

        let rate = get_function("rate").unwrap();
        assert_eq!(rate.variadic, 0);
        assert!(!rate.experimental);
    }
}
