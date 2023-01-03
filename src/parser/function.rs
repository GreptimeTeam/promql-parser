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

use lazy_static::lazy_static;

use crate::parser::ValueType;

#[derive(Debug, Clone)]
pub struct Function {
    pub name: &'static str,
    pub arg_types: Vec<ValueType>,
    pub variadic: bool,
    pub return_type: ValueType,
}

impl Function {
    pub fn new(
        name: &'static str,
        arg_types: Vec<ValueType>,
        variadic: bool,
        return_type: ValueType,
    ) -> Self {
        Self {
            name,
            arg_types,
            variadic,
            return_type,
        }
    }
}

lazy_static! {
    static ref FUNCTIONS: HashMap<&'static str, Function> = {
        let mut m = HashMap::new();

        m.insert(
            "abs",
            Function::new("abs", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "absent",
            Function::new("absent", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "absent_over_time",
            Function::new(
                "absent_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "acos",
            Function::new("acos", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "acosh",
            Function::new("acosh", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "asin",
            Function::new("asin", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "asinh",
            Function::new("asinh", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "atan",
            Function::new("atan", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "atanh",
            Function::new("atanh", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "avg_over_time",
            Function::new(
                "avg_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "ceil",
            Function::new("ceil", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "changes",
            Function::new("changes", vec![ValueType::Matrix], false, ValueType::Vector),
        );

        m.insert(
            "clamp",
            Function::new(
                "clamp",
                vec![ValueType::Vector, ValueType::Scalar, ValueType::Scalar],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "clamp_max",
            Function::new(
                "clamp_max",
                vec![ValueType::Vector, ValueType::Scalar],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "clamp_min",
            Function::new(
                "clamp_min",
                vec![ValueType::Vector, ValueType::Scalar],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "cos",
            Function::new("cos", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "cosh",
            Function::new("cosh", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "count_over_time",
            Function::new(
                "count_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "days_in_month",
            Function::new(
                "days_in_month",
                vec![ValueType::Vector],
                true,
                ValueType::Vector,
            ),
        );

        m.insert(
            "day_of_month",
            Function::new(
                "day_of_month",
                vec![ValueType::Vector],
                true,
                ValueType::Vector,
            ),
        );

        m.insert(
            "day_of_week",
            Function::new(
                "day_of_week",
                vec![ValueType::Vector],
                true,
                ValueType::Vector,
            ),
        );

        m.insert(
            "day_of_year",
            Function::new(
                "day_of_year",
                vec![ValueType::Vector],
                true,
                ValueType::Vector,
            ),
        );

        m.insert(
            "deg",
            Function::new("deg", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "delta",
            Function::new("delta", vec![ValueType::Matrix], false, ValueType::Vector),
        );

        m.insert(
            "deriv",
            Function::new("deriv", vec![ValueType::Matrix], false, ValueType::Vector),
        );

        m.insert(
            "exp",
            Function::new("exp", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "floor",
            Function::new("floor", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "histogram_count",
            Function::new(
                "histogram_count",
                vec![ValueType::Vector],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "histogram_sum",
            Function::new(
                "histogram_sum",
                vec![ValueType::Vector],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "histogram_fraction",
            Function::new(
                "histogram_fraction",
                vec![ValueType::Scalar, ValueType::Scalar, ValueType::Vector],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "histogram_quantile",
            Function::new(
                "histogram_quantile",
                vec![ValueType::Scalar, ValueType::Vector],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "holt_winters",
            Function::new(
                "holt_winters",
                vec![ValueType::Matrix, ValueType::Scalar, ValueType::Scalar],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "hour",
            Function::new("hour", vec![ValueType::Vector], true, ValueType::Vector),
        );

        m.insert(
            "idelta",
            Function::new("idelta", vec![ValueType::Matrix], false, ValueType::Vector),
        );

        m.insert(
            "increase",
            Function::new(
                "increase",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "irate",
            Function::new("irate", vec![ValueType::Matrix], false, ValueType::Vector),
        );

        m.insert(
            "label_replace",
            Function::new(
                "label_replace",
                vec![
                    ValueType::Vector,
                    ValueType::String,
                    ValueType::String,
                    ValueType::String,
                    ValueType::String,
                ],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "label_join",
            Function::new(
                "label_join",
                vec![
                    ValueType::Vector,
                    ValueType::String,
                    ValueType::String,
                    ValueType::String,
                ],
                true,
                ValueType::Vector,
            ),
        );

        m.insert(
            "last_over_time",
            Function::new(
                "last_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "ln",
            Function::new("ln", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "log10",
            Function::new("log10", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "log2",
            Function::new("log2", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "max_over_time",
            Function::new(
                "max_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "min_over_time",
            Function::new(
                "min_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "minute",
            Function::new("minute", vec![ValueType::Vector], true, ValueType::Vector),
        );

        m.insert(
            "month",
            Function::new("month", vec![ValueType::Vector], true, ValueType::Vector),
        );

        m.insert("pi", Function::new("pi", vec![], false, ValueType::Scalar));

        m.insert(
            "predict_linear",
            Function::new(
                "predict_linear",
                vec![ValueType::Matrix, ValueType::Scalar],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "present_over_time",
            Function::new(
                "present_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "quantile_over_time",
            Function::new(
                "quantile_over_time",
                vec![ValueType::Scalar, ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "rad",
            Function::new("rad", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "rate",
            Function::new("rate", vec![ValueType::Matrix], false, ValueType::Vector),
        );

        m.insert(
            "resets",
            Function::new("resets", vec![ValueType::Matrix], false, ValueType::Vector),
        );

        m.insert(
            "round",
            Function::new(
                "round",
                vec![ValueType::Vector, ValueType::Scalar],
                true,
                ValueType::Vector,
            ),
        );

        m.insert(
            "scalar",
            Function::new("scalar", vec![ValueType::Vector], false, ValueType::Scalar),
        );

        m.insert(
            "sgn",
            Function::new("sgn", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "sin",
            Function::new("sin", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "sinh",
            Function::new("sinh", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "sort",
            Function::new("sort", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "sort_desc",
            Function::new(
                "sort_desc",
                vec![ValueType::Vector],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "sqrt",
            Function::new("sqrt", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "stddev_over_time",
            Function::new(
                "stddev_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "stdvar_over_time",
            Function::new(
                "stdvar_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "sum_over_time",
            Function::new(
                "sum_over_time",
                vec![ValueType::Matrix],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "tan",
            Function::new("tan", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "tanh",
            Function::new("tanh", vec![ValueType::Vector], false, ValueType::Vector),
        );

        m.insert(
            "time",
            Function::new("time", vec![], false, ValueType::Scalar),
        );

        m.insert(
            "timestamp",
            Function::new(
                "timestamp",
                vec![ValueType::Vector],
                false,
                ValueType::Vector,
            ),
        );

        m.insert(
            "vector",
            Function::new("vector", vec![ValueType::Scalar], false, ValueType::Vector),
        );

        m.insert(
            "year",
            Function::new("year", vec![ValueType::Vector], true, ValueType::Vector),
        );

        m
    };
}

// get_function returns a predefined Function object for the given name.
pub fn get_function(name: &str) -> Option<&Function> {
    FUNCTIONS.get(name)
}
