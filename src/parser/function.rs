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

use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;

use crate::parser::ValueType;

/// Functions is a list of all functions supported by PromQL, including their types.
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

macro_rules! map {
    ($(($name:literal, $arg:expr, $ret:expr)),*) => (
        {
            let mut m:HashMap<&'static str, Function> = HashMap::new();
            $(
                let variadic = FUNCTIONS_WITH_VARIADIC_ARGS.contains($name);
                let func = Function::new($name, $arg, variadic, $ret);
                m.insert($name, func);
            )*
            m
        }
    );
}

lazy_static! {
    static ref FUNCTIONS_WITH_VARIADIC_ARGS: HashSet<&'static str> = HashSet::from([
        "days_in_month",
        "day_of_year",
        "day_of_month",
        "day_of_week",

        "year",
        "month",
        "hour",
        "minute",

        "label_join",
        "round",
    ]);

    static ref FUNCTIONS: HashMap<&'static str, Function> = map!(
        ("abs", vec![ValueType::Vector], ValueType::Vector),
        ("absent", vec![ValueType::Vector], ValueType::Vector),
        ("absent_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("acos", vec![ValueType::Vector], ValueType::Vector),
        ("acosh", vec![ValueType::Vector], ValueType::Vector),
        ("asin", vec![ValueType::Vector], ValueType::Vector),
        ("asinh", vec![ValueType::Vector], ValueType::Vector),
        ("atan", vec![ValueType::Vector], ValueType::Vector),
        ("atanh", vec![ValueType::Vector], ValueType::Vector),
        ("avg_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("ceil", vec![ValueType::Vector], ValueType::Vector),
        ("changes", vec![ValueType::Matrix], ValueType::Vector),
        ("clamp", vec![ValueType::Vector, ValueType::Scalar, ValueType::Scalar], ValueType::Vector),
        ("clamp_max", vec![ValueType::Vector, ValueType::Scalar], ValueType::Vector),
        ("clamp_min", vec![ValueType::Vector, ValueType::Scalar], ValueType::Vector),
        ("cos", vec![ValueType::Vector], ValueType::Vector),
        ("cosh", vec![ValueType::Vector], ValueType::Vector),
        ("count_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("days_in_month", vec![ValueType::Vector], ValueType::Vector),
        ("day_of_month", vec![ValueType::Vector], ValueType::Vector),
        ("day_of_week", vec![ValueType::Vector], ValueType::Vector),
        ("day_of_year", vec![ValueType::Vector], ValueType::Vector),
        ("deg", vec![ValueType::Vector], ValueType::Vector),
        ("delta", vec![ValueType::Matrix], ValueType::Vector),
        ("deriv", vec![ValueType::Matrix], ValueType::Vector),
        ("exp", vec![ValueType::Vector], ValueType::Vector),
        ("floor", vec![ValueType::Vector], ValueType::Vector),
        ("histogram_count", vec![ValueType::Vector], ValueType::Vector),
        ("histogram_sum", vec![ValueType::Vector], ValueType::Vector),
        ("histogram_fraction", vec![ValueType::Scalar, ValueType::Scalar, ValueType::Vector], ValueType::Vector),
        ("histogram_quantile", vec![ValueType::Scalar, ValueType::Vector], ValueType::Vector),
        ("holt_winters", vec![ValueType::Matrix, ValueType::Scalar, ValueType::Scalar], ValueType::Vector),
        ("hour", vec![ValueType::Vector], ValueType::Vector),
        ("idelta", vec![ValueType::Matrix], ValueType::Vector),
        ("increase", vec![ValueType::Matrix], ValueType::Vector),
        ("irate", vec![ValueType::Matrix], ValueType::Vector),
        (
            "label_replace",
            vec![ValueType::Vector, ValueType::String, ValueType::String, ValueType::String, ValueType::String,],
            ValueType::Vector
        ),
        (
            "label_join", // -1 in Go version
            vec![ValueType::Vector, ValueType::String, ValueType::String, ValueType::String,],
            ValueType::Vector
        ),
        ("last_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("ln", vec![ValueType::Vector], ValueType::Vector),
        ("log10", vec![ValueType::Vector], ValueType::Vector),
        ("log2", vec![ValueType::Vector], ValueType::Vector),
        ("max_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("min_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("minute", vec![ValueType::Vector], ValueType::Vector),
        ("month", vec![ValueType::Vector], ValueType::Vector),
        ("pi", vec![], ValueType::Scalar),
        ("predict_linear", vec![ValueType::Matrix, ValueType::Scalar], ValueType::Vector),
        ("present_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("quantile_over_time", vec![ValueType::Scalar, ValueType::Matrix], ValueType::Vector),
        ("rad", vec![ValueType::Vector], ValueType::Vector),
        ("rate", vec![ValueType::Matrix], ValueType::Vector),
        ("resets", vec![ValueType::Matrix], ValueType::Vector),
        ("round", vec![ValueType::Vector, ValueType::Scalar], ValueType::Vector),
        ("scalar", vec![ValueType::Vector], ValueType::Scalar),
        ("sgn", vec![ValueType::Vector], ValueType::Vector),
        ("sin", vec![ValueType::Vector], ValueType::Vector),
        ("sinh", vec![ValueType::Vector], ValueType::Vector),
        ("sort", vec![ValueType::Vector], ValueType::Vector),
        ("sort_desc", vec![ValueType::Vector], ValueType::Vector),
        ("sqrt", vec![ValueType::Vector], ValueType::Vector),
        ("stddev_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("stdvar_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("sum_over_time", vec![ValueType::Matrix], ValueType::Vector),
        ("tan", vec![ValueType::Vector], ValueType::Vector),
        ("tanh", vec![ValueType::Vector], ValueType::Vector),
        ("time", vec![], ValueType::Scalar),
        ("timestamp", vec![ValueType::Vector], ValueType::Vector),
        ("vector", vec![ValueType::Scalar], ValueType::Vector),
        ("year", vec![ValueType::Vector], ValueType::Vector)
    );
}

// get_function returns a predefined Function object for the given name.
pub fn get_function(name: &str) -> Option<Function> {
    FUNCTIONS.get(name).cloned()
}
