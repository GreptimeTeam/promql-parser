use std::fmt::{self, Display};

/// NORMAL_NAN is a quiet NaN. This is also math.NaN().
pub const NORMAL_NAN: f64 = f64::NAN;

/// STALE_NAN is a signaling NAN, due to the MSB of the mantissa being 0.
/// This value is chosen with many leading 0s, so we have scope to store more
/// complicated values in the future. It is 2 rather than 1 to make
/// it easier to distinguish from the NORMAL_NAN by a human when debugging.
const STALE_NAN_INTEGER: u64 = 0x7ff0000000000002;
pub const STALE_NAN: f64 = STALE_NAN_INTEGER as f64;
pub const STALE_STR: &'static str = "stale";

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

/// IS_STALE_NAN returns true when the provided NaN value is a stale marker.
pub fn is_stale_nan(v: f64) -> bool {
    v == STALE_NAN
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
