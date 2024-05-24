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

/// parse str radix from golang format, but: if 8 or 9 is included
/// in octal literal, it will be treated as decimal literal.
/// This function panics if str is not dec, oct, hex format
///
/// Also accept format like
pub fn parse_str_radix(s: &str) -> Result<f64, String> {
    let st: String = s
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| !c.is_whitespace() && *c != '_')
        .collect();

    let mut is_not_decimal = false;
    if st.contains('x') {
        is_not_decimal = true;
    }

    if !is_not_decimal
        && (st.starts_with("-0") || st.starts_with("+0") || st.starts_with('0'))
        && !st.contains('.')
        && !st.contains('8')
        && !st.contains('9')
        && !st.eq("0")
        && !st.eq("+0")
        && !st.eq("-0")
    {
        is_not_decimal = true;
    }

    if is_not_decimal {
        let i = if st.starts_with("-0x") {
            i64::from_str_radix(st.strip_prefix("-0x").unwrap(), 16).map(|x| -x)
        } else if st.starts_with("+0x") {
            i64::from_str_radix(st.strip_prefix("+0x").unwrap(), 16)
        } else if st.starts_with("0x") {
            i64::from_str_radix(st.strip_prefix("0x").unwrap(), 16)
        } else if st.starts_with("-0") {
            i64::from_str_radix(st.strip_prefix("-0").unwrap(), 8).map(|x| -x)
        } else if st.starts_with("+0") {
            i64::from_str_radix(st.strip_prefix("+0").unwrap(), 8)
        } else {
            i64::from_str_radix(st.strip_prefix('0').unwrap(), 8) // starts with '0'
        };
        return i
            .map(|x| x as f64)
            .map_err(|_| format!("ParseFloatError. {s} can't be parsed into i64"));
    }
    if let Some(s) = st.strip_suffix('k') {
        s.parse().map(|s: f64| s * 1000_f64)
    } else if let Some(s) = st.strip_suffix("ki") {
        s.parse().map(|s: f64| s * 1024_f64)
    } else if let Some(s) = st.strip_suffix('m') {
        s.parse().map(|s: f64| s * 1000_f64.powi(2))
    } else if let Some(s) = st.strip_suffix("mi") {
        s.parse().map(|s: f64| s * 1024_f64.powi(2))
    } else if let Some(s) = st.strip_suffix('g') {
        s.parse().map(|s: f64| s * 1000_f64.powi(3))
    } else if let Some(s) = st.strip_suffix("gi") {
        s.parse().map(|s: f64| s * 1024_f64.powi(3))
    } else if let Some(s) = st.strip_suffix('t') {
        s.parse().map(|s: f64| s * 1000_f64.powi(4))
    } else if let Some(s) = st.strip_suffix("ti") {
        s.parse().map(|s: f64| s * 1024_f64.powi(4))
    } else {
        st.parse()
    }
    .map_err(|_| format!("ParseFloatError. {s} can't be parsed into f64"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_str_radix() {
        assert_eq!(parse_str_radix("1K").unwrap(), 1000_f64);
        assert_eq!(parse_str_radix("1Ki").unwrap(), 1024_f64);
        assert_eq!(parse_str_radix("1M").unwrap(), 1000_f64.powi(2));
        assert_eq!(parse_str_radix("1Mi").unwrap(), 1024_f64.powi(2));
        assert_eq!(parse_str_radix("1G").unwrap(), 1000_f64.powi(3));
        assert_eq!(parse_str_radix("1Gi").unwrap(), 1024_f64.powi(3));
        assert_eq!(parse_str_radix("1T").unwrap(), 1000_f64.powi(4));
        assert_eq!(parse_str_radix("1Ti").unwrap(), 1024_f64.powi(4));
        assert_eq!(parse_str_radix("10_20_30.10_2").unwrap(), 102030.102_f64);
        assert_eq!(parse_str_radix("0x2f").unwrap(), 47_f64);
        assert_eq!(parse_str_radix("+0x2f").unwrap(), 47_f64);
        assert_eq!(parse_str_radix("- 0x2f ").unwrap(), -47_f64);
        assert_eq!(parse_str_radix("017").unwrap(), 15_f64);
        assert_eq!(parse_str_radix("-017").unwrap(), -15_f64);
        assert_eq!(parse_str_radix("+017").unwrap(), 15_f64);
        assert_eq!(parse_str_radix("2023.0128").unwrap(), 2023.0128_f64);
        assert_eq!(parse_str_radix("-4.14").unwrap(), -4.14_f64);
        assert_eq!(parse_str_radix("+3.718").unwrap(), 3.718_f64);
        assert_eq!(parse_str_radix("-0.14").unwrap(), -0.14_f64);
        assert_eq!(parse_str_radix("+0.718").unwrap(), 0.718_f64);
        assert_eq!(parse_str_radix("0.718").unwrap(), 0.718_f64);
        assert_eq!(parse_str_radix("089").unwrap(), 89_f64);
        assert_eq!(parse_str_radix("+089").unwrap(), 89_f64);
        assert_eq!(parse_str_radix("-089").unwrap(), -89_f64);
        assert_eq!(parse_str_radix("+0").unwrap(), 0_f64);
        assert_eq!(parse_str_radix("-0").unwrap(), 0_f64);
        assert_eq!(parse_str_radix("0").unwrap(), 0_f64);

        assert!(parse_str_radix("rust").is_err());
        assert!(parse_str_radix("0xgolang").is_err());
        assert!(parse_str_radix("0clojure").is_err());
        assert!(parse_str_radix("0x2024Ti").is_err());
    }
}
