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

//! Internal utilities for strings.

/// This function is modified from original go version
pub fn unquote_string(s: &str) -> Result<String, String> {
    let n = s.len();
    if n < 2 {
        return Err("invalid syntax".to_string());
    }

    let bytes = s.as_bytes();
    let quote = bytes[0];
    if quote != bytes[n - 1] {
        return Err("invalid syntax".to_string());
    }

    let inner = &s[1..n - 1];

    if quote == b'`' {
        if inner.contains('`') {
            return Err("invalid syntax".to_string());
        }
        return Ok(inner.to_string());
    }

    if quote != b'"' && quote != b'\'' {
        return Err("invalid syntax".to_string());
    }

    if inner.contains('\n') {
        return Err("invalid syntax".to_string());
    }

    if !inner.contains('\\') && !inner.contains(quote as char) {
        return Ok(inner.to_string());
    }

    let mut res = String::with_capacity(3 * inner.len() / 2);
    let mut rest = inner;

    while !rest.is_empty() {
        let (c, tail) = unquote_char(rest, quote)?;
        res.push(c);
        rest = tail;
    }

    Ok(res)
}

fn unquote_char(s: &str, quote: u8) -> Result<(char, &str), String> {
    let bytes = s.as_bytes();
    let c = bytes[0];

    // Easy cases
    if c == quote && (quote == b'\'' || quote == b'"') {
        return Err("invalid syntax".to_string());
    }

    if c < 0x80 {
        if c != b'\\' {
            return Ok((c as char, &s[1..]));
        }
    } else {
        // Handle multi-byte UTF-8 character
        let r = s.chars().next().unwrap();
        return Ok((r, &s[r.len_utf8()..]));
    }

    // Hard case: backslash
    if s.len() <= 1 {
        return Err("invalid syntax".to_string());
    }

    let c = bytes[1];
    let mut tail = &s[2..];

    let value = match c {
        b'a' => '\x07', // Alert/Bell
        b'b' => '\x08', // Backspace
        b'f' => '\x0c', // Form feed
        b'n' => '\n',
        b'r' => '\r',
        b't' => '\t',
        b'v' => '\x0b', // Vertical tab
        b'x' | b'u' | b'U' => {
            let n = match c {
                b'x' => 2,
                b'u' => 4,
                b'U' => 8,
                _ => unreachable!(),
            };

            if tail.len() < n {
                return Err("invalid syntax".to_string());
            }

            let mut v: u32 = 0;
            for i in 0..n {
                let x = unhex(tail.as_bytes()[i])?;
                v = (v << 4) | x;
            }

            tail = &tail[n..];

            if c == b'x' {
                std::char::from_u32(v).ok_or("invalid syntax")?
            } else {
                if v > 0x10FFFF {
                    return Err("invalid syntax".to_string());
                }
                std::char::from_u32(v).ok_or("invalid syntax")?
            }
        }
        b'0'..=b'7' => {
            let mut v = (c - b'0') as u32;
            if tail.len() < 2 {
                return Err("invalid syntax".to_string());
            }
            for i in 0..2 {
                let x = (tail.as_bytes()[i] as char)
                    .to_digit(8)
                    .ok_or("invalid syntax")?;
                v = (v << 3) | x;
            }
            tail = &tail[2..];
            if v > 255 {
                return Err("invalid syntax".to_string());
            }
            std::char::from_u32(v).ok_or("invalid syntax")?
        }
        b'\\' => '\\',
        b'\'' | b'"' => {
            if c != quote {
                return Err("invalid syntax".to_string());
            }
            c as char
        }
        _ => return Err("invalid syntax".to_string()),
    };

    Ok((value, tail))
}

fn unhex(b: u8) -> Result<u32, String> {
    let c = b as char;
    c.to_digit(16).ok_or_else(|| "invalid syntax".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unquote_string_basic() {
        // Test simple double quotes
        assert_eq!(unquote_string("\"hello\"").unwrap(), "hello");

        // Test simple single quotes
        assert_eq!(unquote_string("'hello'").unwrap(), "hello");

        // Test backticks
        assert_eq!(unquote_string("`hello`").unwrap(), "hello");
    }

    #[test]
    fn test_unquote_string_empty() {
        assert_eq!(unquote_string("\"\"").unwrap(), "");
        assert_eq!(unquote_string("''").unwrap(), "");
        assert_eq!(unquote_string("``").unwrap(), "");
    }

    #[test]
    fn test_unquote_string_error_cases() {
        // Too short
        assert!(unquote_string("\"").is_err());
        assert!(unquote_string("'").is_err());
        assert!(unquote_string("`").is_err());

        // Mismatched quotes
        assert!(unquote_string("\"hello'").is_err());
        assert!(unquote_string("'hello\"").is_err());
        assert!(unquote_string("`hello\"").is_err());

        // Invalid quote character
        assert!(unquote_string("#hello#").is_err());
        assert!(unquote_string("/hello/").is_err());

        // Newlines in quoted strings
        assert!(unquote_string("\"hello\nworld\"").is_err());
        assert!(unquote_string("'hello\nworld'").is_err());

        // Backticks with backticks inside
        assert!(unquote_string("`hello`world`").is_err());
    }

    #[test]
    fn test_unquote_string_escaped_characters() {
        // Test various escape sequences
        assert_eq!(unquote_string(r#""\a""#).unwrap(), "\x07");
        assert_eq!(unquote_string(r#""\b""#).unwrap(), "\x08");
        assert_eq!(unquote_string(r#""\f""#).unwrap(), "\x0c");
        assert_eq!(unquote_string(r#""\n""#).unwrap(), "\n");
        assert_eq!(unquote_string(r#""\r""#).unwrap(), "\r");
        assert_eq!(unquote_string(r#""\t""#).unwrap(), "\t");
        assert_eq!(unquote_string(r#""\v""#).unwrap(), "\x0b");

        // Test escaped backslashes
        assert_eq!(unquote_string(r#""\\""#).unwrap(), "\\");

        // Test escaped quotes
        assert_eq!(unquote_string(r#""\"""#).unwrap(), "\"");
        assert_eq!(unquote_string(r#"'\''"#).unwrap(), "'");
        assert_eq!(
            unquote_string(r#""double-quoted raw string \" with escaped quote""#).unwrap(),
            "double-quoted raw string \" with escaped quote"
        );

        // Mixed escape sequences
        assert_eq!(unquote_string(r#""hello\nworld""#).unwrap(), "hello\nworld");
        assert_eq!(unquote_string(r#""hello\tworld""#).unwrap(), "hello\tworld");
    }

    #[test]
    fn test_unquote_string_hex_escapes() {
        // Test \x hex escapes
        assert_eq!(unquote_string(r#""\x41""#).unwrap(), "A");
        assert_eq!(unquote_string(r#""\x61""#).unwrap(), "a");
        assert_eq!(unquote_string(r#""\x20""#).unwrap(), " ");

        // Test multiple hex escapes
        assert_eq!(
            unquote_string(r#""\x48\x65\x6c\x6c\x6f""#).unwrap(),
            "Hello"
        );

        // Test invalid hex escapes
        assert!(unquote_string(r#""\x""#).is_err()); // too short
        assert!(unquote_string(r#""\x4""#).is_err()); // too short
        assert!(unquote_string(r#""\x4G""#).is_err()); // invalid hex digit
    }

    #[test]
    fn test_unquote_string_unicode_escapes() {
        // Test \u unicode escapes (4 digits)
        assert_eq!(unquote_string(r#""\u0041""#).unwrap(), "A");
        assert_eq!(unquote_string(r#""\u0061""#).unwrap(), "a");
        assert_eq!(unquote_string(r#""\u20AC""#).unwrap(), "€"); // Euro sign

        // Test \U unicode escapes (8 digits)
        assert_eq!(unquote_string(r#""\U00000041""#).unwrap(), "A");
        assert_eq!(unquote_string(r#""\U00000061""#).unwrap(), "a");
        assert_eq!(unquote_string(r#""\U000020AC""#).unwrap(), "€"); // Euro sign

        // Test invalid unicode escapes
        assert!(unquote_string(r#""\u""#).is_err()); // too short
        assert!(unquote_string(r#""\u123""#).is_err()); // too short
        assert!(unquote_string(r#""\U""#).is_err()); // too short
        assert!(unquote_string(r#""\U1234567""#).is_err()); // too short
        assert!(unquote_string(r#""\U11000000""#).is_err()); // beyond Unicode range
    }

    #[test]
    fn test_unquote_string_octal_escapes() {
        // Test octal escapes
        assert_eq!(unquote_string(r#""\101""#).unwrap(), "A"); // 101 octal = 65 decimal = 'A'
        assert_eq!(unquote_string(r#""\141""#).unwrap(), "a"); // 141 octal = 97 decimal = 'a'
        assert_eq!(unquote_string(r#""\040""#).unwrap(), " "); // 040 octal = 32 decimal = space

        // Test invalid octal escapes
        assert!(unquote_string(r#""\1""#).is_err()); // too short
        assert!(unquote_string(r#""\12""#).is_err()); // too short
        assert!(unquote_string(r#""\400""#).is_err()); // 400 octal = 256 decimal > 255
        assert!(unquote_string(r#""\8""#).is_err()); // invalid octal digit
    }

    #[test]
    fn test_unquote_string_utf8_characters() {
        // Test multi-byte UTF-8 characters
        assert_eq!(unquote_string("\"café\"").unwrap(), "café");
        assert_eq!(unquote_string("\"🦀\"").unwrap(), "🦀");
        assert_eq!(unquote_string("\"こんにちは\"").unwrap(), "こんにちは");
    }

    #[test]
    fn test_unquote_string_mixed_content() {
        // Test strings with mixed content
        assert_eq!(
            unquote_string(r#""Hello, \u4e16\u754c!""#).unwrap(),
            "Hello, 世界!"
        );
        assert_eq!(
            unquote_string(r#""Line1\nLine2\tEnd""#).unwrap(),
            "Line1\nLine2\tEnd"
        );
        assert_eq!(
            unquote_string(r#""Path: C:\\\\Windows\\\\System32""#).unwrap(),
            "Path: C:\\\\Windows\\\\System32"
        );
    }

    #[test]
    fn test_unquote_string_edge_cases() {
        // Test quote character inside string without escape (should work if same as outer quote)
        assert_eq!(unquote_string(r#"'It"s'"#).unwrap(), "It\"s");

        // Test escaped quote that doesn't match outer quote (should fail)
        assert!(unquote_string(r#""\'"'"#).is_err()); // trying to escape single quote in double quotes

        // Test single quote with escaped single quote (should work)
        assert_eq!(unquote_string(r#"'\''"#).unwrap(), "'");

        // Test empty escape at end
        assert!(unquote_string(r#""\""#).is_err());
    }

    #[test]
    fn test_unquote_string_complex_escape_sequences() {
        // Test complex combination of escape sequences
        let complex = r#""Hello\x20World\n\u4e16\u754c\t\U0001F600""#;
        let expected = "Hello World\n世界\t😀";
        assert_eq!(unquote_string(complex).unwrap(), expected);
    }

    #[test]
    fn test_unquote_string_backtick_edge_cases() {
        // Test backticks with various content
        assert_eq!(unquote_string("`hello world`").unwrap(), "hello world");
        assert_eq!(unquote_string("`hello\nworld`").unwrap(), "hello\nworld"); // newlines allowed in backticks
        assert_eq!(unquote_string("`hello\\nworld`").unwrap(), "hello\\nworld"); // backslashes treated literally

        // Test nested backticks (should fail)
        assert!(unquote_string("`hello`world`").is_err());
        assert!(unquote_string("``hello`").is_err());
    }
}
