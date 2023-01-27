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

/// parse str radix from golang format
pub fn parse_golang_str_radix(s: &str) -> Result<f64, String> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(s.to_lowercase().strip_prefix("0x").unwrap(), 16)
            .map(|x| x as f64)
            .map_err(|_| format!("ParseFloatError. {} can't be parsed into f64", s))
    } else if s.starts_with("0") {
        u64::from_str_radix(s.strip_prefix("0").unwrap(), 8)
            .map(|x| x as f64)
            .map_err(|_| format!("ParseFloatError. {} can't be parsed into f64", s))
    } else {
        s.parse::<f64>()
            .map_err(|_| format!("ParseFloatError. {} can't be parsed into f64", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_golang_str_radix() {
        assert_eq!(15f64, parse_golang_str_radix("0xf").unwrap());
        assert_eq!(7f64, parse_golang_str_radix("07").unwrap());
        assert_eq!(7f64, parse_golang_str_radix("7").unwrap());
        assert_eq!(-7f64, parse_golang_str_radix("-7").unwrap());
    }
}
