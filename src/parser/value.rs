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
pub enum ValueType {
    Vector,
    Scalar,
    Matrix,
    String,
    None,
}

impl Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ValueType::None => write!(f, "none"),
            ValueType::Scalar => write!(f, "scalar"),
            ValueType::String => write!(f, "string"),
            ValueType::Vector => write!(f, "instant vector"),
            ValueType::Matrix => write!(f, "range vector"),
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
        assert_eq!(ValueType::None.to_string(), "none");
        assert_eq!(ValueType::Scalar.to_string(), "scalar");
        assert_eq!(ValueType::String.to_string(), "string");
        assert_eq!(ValueType::Vector.to_string(), "instant vector");
        assert_eq!(ValueType::Matrix.to_string(), "range vector");
    }
}
