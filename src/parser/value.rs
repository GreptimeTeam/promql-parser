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

use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ser", derive(serde::Serialize))]
#[cfg_attr(feature = "ser", serde(rename_all = "camelCase"))]
pub enum ValueType {
    Vector,
    Scalar,
    Matrix,
    String,
}

impl Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ValueType::Scalar => write!(f, "scalar"),
            ValueType::String => write!(f, "string"),
            ValueType::Vector => write!(f, "vector"),
            ValueType::Matrix => write!(f, "matrix"),
        }
    }
}

pub trait Value {
    fn vtype(&self) -> ValueType;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type() {
        assert_eq!(ValueType::Scalar.to_string(), "scalar");
        assert_eq!(ValueType::String.to_string(), "string");
        assert_eq!(ValueType::Vector.to_string(), "vector");
        assert_eq!(ValueType::Matrix.to_string(), "matrix");
    }
}
