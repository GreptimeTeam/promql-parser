use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq)]
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
        assert_eq!(ValueType::Scalar.to_string(), "scalar");
        assert_eq!(ValueType::String.to_string(), "string");
        assert_eq!(ValueType::Vector.to_string(), "instant vector");
        assert_eq!(ValueType::Matrix.to_string(), "range vector");
    }
}
