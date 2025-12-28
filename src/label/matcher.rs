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

use std::fmt;
use std::hash::{Hash, Hasher};

use regex::Regex;

use crate::parser::token::{token_display, TokenId, T_EQL, T_EQL_REGEX, T_NEQ, T_NEQ_REGEX};
use crate::util::join_vector;

const LABEL_METRIC_NAME: &str = "__name__";

#[derive(Debug, Clone)]
pub enum MatchOp {
    Equal,
    NotEqual,
    // TODO: do we need regex here?
    Re(Regex),
    NotRe(Regex),
}

impl fmt::Display for MatchOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatchOp::Equal => write!(f, "="),
            MatchOp::NotEqual => write!(f, "!="),
            MatchOp::Re(_reg) => write!(f, "=~"),
            MatchOp::NotRe(_reg) => write!(f, "!~"),
        }
    }
}

impl PartialEq for MatchOp {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MatchOp::Equal, MatchOp::Equal) => true,
            (MatchOp::NotEqual, MatchOp::NotEqual) => true,
            (MatchOp::Re(s), MatchOp::Re(o)) => s.as_str().eq(o.as_str()),
            (MatchOp::NotRe(s), MatchOp::NotRe(o)) => s.as_str().eq(o.as_str()),
            _ => false,
        }
    }
}

impl Eq for MatchOp {}

impl Hash for MatchOp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            MatchOp::Equal => "eq".hash(state),
            MatchOp::NotEqual => "ne".hash(state),
            MatchOp::Re(s) => format!("re:{}", s.as_str()).hash(state),
            MatchOp::NotRe(s) => format!("nre:{}", s.as_str()).hash(state),
        }
    }
}

#[cfg(feature = "ser")]
impl serde::Serialize for MatchOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// Matcher models the matching of a label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ser", derive(serde::Serialize))]
pub struct Matcher {
    #[cfg_attr(feature = "ser", serde(rename = "type"))]
    pub op: MatchOp,
    pub name: String,
    pub value: String,
}

impl Matcher {
    pub fn new(op: MatchOp, name: &str, value: &str) -> Self {
        Self {
            op,
            name: name.into(),
            value: value.into(),
        }
    }

    /// matches returns whether the matcher matches the given string value.
    pub fn is_match(&self, s: &str) -> bool {
        match &self.op {
            MatchOp::Equal => self.value.eq(s),
            MatchOp::NotEqual => self.value.ne(s),
            MatchOp::Re(r) => r.is_match(s),
            MatchOp::NotRe(r) => !r.is_match(s),
        }
    }

    /// Parse and potentially transform the regex.
    ///
    /// Go and Rust handle the repeat pattern differently,
    /// in Go the following is valid: `aaa{bbb}ccc` but
    /// in Rust {bbb} is seen as an invalid repeat and must be escaped as \{bbb}.
    /// This escapes the opening { if its not followed by valid repeat pattern (e.g. 4,6).
    ///
    /// Regex used in PromQL are fully anchored.
    fn try_parse_re(original_re: &str) -> Result<Regex, String> {
        let re = format!(
            "^(?:{})$",
            unescaper::unescape(original_re).map_err(|e| format!("Invalid regex pattern, {e}"))?
        );
        Regex::new(&re)
            .or_else(|_| Regex::new(&try_escape_for_repeat_re(&re)))
            .map_err(|_| format!("illegal regex for {original_re}",))
    }

    pub fn new_matcher(id: TokenId, name: String, value: String) -> Result<Matcher, String> {
        let op = Self::find_matcher_op(id, &value)?;
        op.map(|op| Matcher::new(op, name.as_str(), value.as_str()))
    }

    pub fn new_metric_name_matcher(name: String) -> Result<Matcher, String> {
        Ok(Matcher::new(
            MatchOp::Equal,
            LABEL_METRIC_NAME,
            name.as_str(),
        ))
    }

    fn find_matcher_op(id: TokenId, value: &str) -> Result<Result<MatchOp, String>, String> {
        let op = match id {
            T_EQL => Ok(MatchOp::Equal),
            T_NEQ => Ok(MatchOp::NotEqual),
            T_EQL_REGEX => Ok(MatchOp::Re(Matcher::try_parse_re(value)?)),
            T_NEQ_REGEX => Ok(MatchOp::NotRe(Matcher::try_parse_re(value)?)),
            _ => Err(format!("invalid match op {}", token_display(id))),
        };
        Ok(op)
    }
}

impl fmt::Display for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}\"{}\"", self.name, self.op, self.value)
    }
}

// Go and Rust handle the repeat pattern differently
// in Go the following is valid: `aaa{bbb}ccc`
// in Rust {bbb} is seen as an invalid repeat and must be ecaped \{bbb}
// This escapes the opening { if its not followed by valid repeat pattern (e.g. 4,6).
fn try_escape_for_repeat_re(re: &str) -> String {
    fn is_repeat(chars: &mut std::str::Chars<'_>) -> (bool, String) {
        let mut buf = String::new();
        let mut comma_seen = false;
        for c in chars.by_ref() {
            buf.push(c);
            match c {
                ',' if comma_seen => {
                    return (false, buf); // ,, is invalid
                }
                ',' if buf == "," => {
                    return (false, buf); // {, is invalid
                }
                ',' if !comma_seen => comma_seen = true,
                '}' if buf == "}" => {
                    return (false, buf); // {} is invalid
                }
                '}' => {
                    return (true, buf);
                }
                _ if c.is_ascii_digit() => continue,
                _ => {
                    return (false, buf); // false if visit non-digit char
                }
            }
        }
        (false, buf) // not ended with }
    }

    let mut result = String::with_capacity(re.len() + 1);
    let mut chars = re.chars();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(cc) = chars.next() {
                    result.push(c);
                    result.push(cc);
                }
            }
            '{' => {
                let (is, s) = is_repeat(&mut chars);
                if !is {
                    result.push('\\');
                }
                result.push(c);
                result.push_str(&s);
            }
            _ => result.push(c),
        }
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ser", derive(serde::Serialize))]
pub struct Matchers {
    pub matchers: Vec<Matcher>,
    #[cfg_attr(feature = "ser", serde(skip_serializing_if = "<[_]>::is_empty"))]
    pub or_matchers: Vec<Vec<Matcher>>,
}

impl Matchers {
    pub fn empty() -> Self {
        Self {
            matchers: vec![],
            or_matchers: vec![],
        }
    }

    pub fn one(matcher: Matcher) -> Self {
        let matchers = vec![matcher];
        Self {
            matchers,
            or_matchers: vec![],
        }
    }

    pub fn new(matchers: Vec<Matcher>) -> Self {
        Self {
            matchers,
            or_matchers: vec![],
        }
    }

    pub fn with_or_matchers(mut self, or_matchers: Vec<Vec<Matcher>>) -> Self {
        self.or_matchers = or_matchers;
        self
    }

    pub fn append(mut self, matcher: Matcher) -> Self {
        // Check the latest or_matcher group. If it is not empty,
        // we need to add the current matcher to this group.
        let last_or_matcher = self.or_matchers.last_mut();
        if let Some(last_or_matcher) = last_or_matcher {
            last_or_matcher.push(matcher);
        } else {
            self.matchers.push(matcher);
        }
        self
    }

    pub fn append_or(mut self, matcher: Matcher) -> Self {
        if !self.matchers.is_empty() {
            // Be careful not to move ownership here, because it
            // will be used by the subsequent append method.
            let last_matchers = std::mem::take(&mut self.matchers);
            self.or_matchers.push(last_matchers);
        }
        let new_or_matchers = vec![matcher];
        self.or_matchers.push(new_or_matchers);
        self
    }

    /// Vector selectors must either specify a name or at least one label
    /// matcher that does not match the empty string.
    ///
    /// The following expression is illegal:
    /// {job=~".*"} # Bad!
    pub fn is_empty_matchers(&self) -> bool {
        (self.matchers.is_empty() && self.or_matchers.is_empty())
            || self
                .matchers
                .iter()
                .chain(self.or_matchers.iter().flatten())
                .all(|m| m.is_match(""))
    }

    /// find the matcher's value whose name equals the specified name. This function
    /// is designed to prepare error message of invalid promql expression.
    pub(crate) fn find_matcher_value(&self, name: &str) -> Option<String> {
        self.matchers
            .iter()
            .chain(self.or_matchers.iter().flatten())
            .find(|m| m.name.eq(name))
            .map(|m| m.value.clone())
    }

    /// find matchers whose name equals the specified name
    pub fn find_matchers(&self, name: &str) -> Vec<Matcher> {
        self.matchers
            .iter()
            .chain(self.or_matchers.iter().flatten())
            .filter(|m| m.name.eq(name))
            .cloned()
            .collect()
    }
}

impl fmt::Display for Matchers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let simple_matchers = &self.matchers;
        let or_matchers = &self.or_matchers;
        if or_matchers.is_empty() {
            write!(f, "{}", join_vector(simple_matchers, ",", true))
        } else {
            let or_matchers_string =
                self.or_matchers
                    .iter()
                    .fold(String::new(), |or_matchers_str, pair| {
                        format!("{} or {}", or_matchers_str, join_vector(pair, ", ", false))
                    });
            let or_matchers_string = or_matchers_string.trim_start_matches(" or").trim();
            write!(f, "{or_matchers_string}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::token;
    use std::collections::hash_map::DefaultHasher;

    fn hash<H>(op: H) -> u64
    where
        H: Hash,
    {
        let mut hasher = DefaultHasher::new();
        op.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_new_matcher() {
        assert_eq!(
            Matcher::new_matcher(token::T_ADD, "".into(), "".into()),
            Err(format!("invalid match op {}", token_display(token::T_ADD)))
        )
    }

    #[test]
    fn test_matcher_op_eq() {
        assert_eq!(MatchOp::Equal, MatchOp::Equal);
        assert_eq!(MatchOp::NotEqual, MatchOp::NotEqual);
        assert_eq!(
            MatchOp::Re(Regex::new("\\s+").unwrap()),
            MatchOp::Re(Regex::new("\\s+").unwrap())
        );
        assert_eq!(
            MatchOp::NotRe(Regex::new("\\s+").unwrap()),
            MatchOp::NotRe(Regex::new("\\s+").unwrap())
        );

        assert_ne!(MatchOp::Equal, MatchOp::NotEqual);
        assert_ne!(
            MatchOp::NotEqual,
            MatchOp::NotRe(Regex::new("\\s+").unwrap())
        );
        assert_ne!(
            MatchOp::Re(Regex::new("\\s+").unwrap()),
            MatchOp::NotRe(Regex::new("\\s+").unwrap())
        );
    }

    #[test]
    fn test_matchop_hash() {
        assert_eq!(hash(MatchOp::Equal), hash(MatchOp::Equal));
        assert_eq!(hash(MatchOp::NotEqual), hash(MatchOp::NotEqual));
        assert_eq!(
            hash(MatchOp::Re(Regex::new("\\s+").unwrap())),
            hash(MatchOp::Re(Regex::new("\\s+").unwrap()))
        );
        assert_eq!(
            hash(MatchOp::NotRe(Regex::new("\\s+").unwrap())),
            hash(MatchOp::NotRe(Regex::new("\\s+").unwrap()))
        );

        assert_ne!(hash(MatchOp::Equal), hash(MatchOp::NotEqual));
        assert_ne!(
            hash(MatchOp::NotEqual),
            hash(MatchOp::NotRe(Regex::new("\\s+").unwrap()))
        );
        assert_ne!(
            hash(MatchOp::Re(Regex::new("\\s+").unwrap())),
            hash(MatchOp::NotRe(Regex::new("\\s+").unwrap()))
        );
    }

    #[test]
    fn test_matcher_hash() {
        assert_eq!(
            hash(Matcher::new(MatchOp::Equal, "name", "value")),
            hash(Matcher::new(MatchOp::Equal, "name", "value")),
        );

        assert_eq!(
            hash(Matcher::new(MatchOp::NotEqual, "name", "value")),
            hash(Matcher::new(MatchOp::NotEqual, "name", "value")),
        );

        assert_eq!(
            hash(Matcher::new(
                MatchOp::Re(Regex::new("\\s+").unwrap()),
                "name",
                "\\s+"
            )),
            hash(Matcher::new(
                MatchOp::Re(Regex::new("\\s+").unwrap()),
                "name",
                "\\s+"
            )),
        );

        assert_eq!(
            hash(Matcher::new(
                MatchOp::NotRe(Regex::new("\\s+").unwrap()),
                "name",
                "\\s+"
            )),
            hash(Matcher::new(
                MatchOp::NotRe(Regex::new("\\s+").unwrap()),
                "name",
                "\\s+"
            )),
        );

        assert_ne!(
            hash(Matcher::new(MatchOp::Equal, "name", "value")),
            hash(Matcher::new(MatchOp::NotEqual, "name", "value")),
        );

        assert_ne!(
            hash(Matcher::new(
                MatchOp::Re(Regex::new("\\s+").unwrap()),
                "name",
                "\\s+"
            )),
            hash(Matcher::new(
                MatchOp::NotRe(Regex::new("\\s+").unwrap()),
                "name",
                "\\s+"
            )),
        );
    }

    #[test]
    fn test_matcher_eq_ne() {
        let op = MatchOp::Equal;
        let matcher = Matcher::new(op, "name", "up");
        assert!(matcher.is_match("up"));
        assert!(!matcher.is_match("down"));

        let op = MatchOp::NotEqual;
        let matcher = Matcher::new(op, "name", "up");
        assert!(matcher.is_match("foo"));
        assert!(matcher.is_match("bar"));
        assert!(!matcher.is_match("up"));
    }

    #[test]
    fn test_matcher_re() {
        let value = "api/v1/.*";
        let re = Regex::new(value).unwrap();
        let op = MatchOp::Re(re);
        let matcher = Matcher::new(op, "name", value);
        assert!(matcher.is_match("api/v1/query"));
        assert!(matcher.is_match("api/v1/range_query"));
        assert!(!matcher.is_match("api/v2"));
    }

    #[test]
    fn test_eq_matcher_equality() {
        assert_eq!(
            Matcher::new(MatchOp::Equal, "code", "200"),
            Matcher::new(MatchOp::Equal, "code", "200")
        );

        assert_ne!(
            Matcher::new(MatchOp::Equal, "code", "200"),
            Matcher::new(MatchOp::Equal, "code", "201")
        );

        assert_ne!(
            Matcher::new(MatchOp::Equal, "code", "200"),
            Matcher::new(MatchOp::NotEqual, "code", "200")
        );
    }

    #[test]
    fn test_ne_matcher_equality() {
        assert_eq!(
            Matcher::new(MatchOp::NotEqual, "code", "200"),
            Matcher::new(MatchOp::NotEqual, "code", "200")
        );

        assert_ne!(
            Matcher::new(MatchOp::NotEqual, "code", "200"),
            Matcher::new(MatchOp::NotEqual, "code", "201")
        );

        assert_ne!(
            Matcher::new(MatchOp::NotEqual, "code", "200"),
            Matcher::new(MatchOp::Equal, "code", "200")
        );
    }

    #[test]
    fn test_re_matcher_equality() {
        assert_eq!(
            Matcher::new(MatchOp::Re(Regex::new("2??").unwrap()), "code", "2??",),
            Matcher::new(MatchOp::Re(Regex::new("2??").unwrap()), "code", "2??",)
        );

        assert_ne!(
            Matcher::new(MatchOp::Re(Regex::new("2??").unwrap()), "code", "2??",),
            Matcher::new(MatchOp::Re(Regex::new("2??").unwrap()), "code", "2*?",)
        );

        assert_ne!(
            Matcher::new(MatchOp::Re(Regex::new("2??").unwrap()), "code", "2??",),
            Matcher::new(MatchOp::Equal, "code", "2??")
        );

        // Test anchoring behavior - should match full string only
        let matcher = Matcher::new(
            MatchOp::Re(Matcher::try_parse_re("abc.*").unwrap()),
            "code",
            "abc.*",
        );
        assert!(matcher.is_match("abc123"));
        assert!(!matcher.is_match("xabc123"));

        let matcher = Matcher::new(
            MatchOp::Re(Matcher::try_parse_re(".*xyz$").unwrap()),
            "code",
            ".*xyz",
        );
        assert!(matcher.is_match("123xyz"));
        assert!(!matcher.is_match("123xyzx"));

        let matcher = Matcher::new(
            MatchOp::Re(Matcher::try_parse_re("abc").unwrap()),
            "code",
            "abc",
        );
        assert!(matcher.is_match("abc"));
        assert!(!matcher.is_match("xabc"));
        assert!(!matcher.is_match("abcx"));

        let matcher = Matcher::new(
            MatchOp::Re(Matcher::try_parse_re("127.0.0.1").unwrap()),
            "code",
            "127.0.0.1",
        );
        assert!(matcher.is_match("127.0.0.1"));
        assert!(!matcher.is_match("x127.0.0.1"));
        assert!(!matcher.is_match("127.0.0.2"));

        let raw_input = r#"127\\.0\\.0\\.1"#;
        let matcher = Matcher::new(
            MatchOp::Re(Matcher::try_parse_re(raw_input).unwrap()),
            "code",
            raw_input,
        );
        assert!(matcher.is_match("127.0.0.1"));
        assert!(!matcher.is_match("x127.0.0.1"));
        assert!(!matcher.is_match("127.0.0.2"));
        // regex round trip
        let re = Matcher::try_parse_re(raw_input).unwrap();
        let new_re = Regex::new(re.as_str()).unwrap();
        assert_eq!(re.as_str(), new_re.as_str());
    }

    #[test]
    fn test_not_re_matcher_equality() {
        assert_eq!(
            Matcher::new(MatchOp::NotRe(Regex::new("2??").unwrap()), "code", "2??",),
            Matcher::new(MatchOp::NotRe(Regex::new("2??").unwrap()), "code", "2??",)
        );

        assert_ne!(
            Matcher::new(MatchOp::NotRe(Regex::new("2??").unwrap()), "code", "2??",),
            Matcher::new(MatchOp::NotRe(Regex::new("2?*").unwrap()), "code", "2*?",)
        );

        assert_ne!(
            Matcher::new(MatchOp::NotRe(Regex::new("2??").unwrap()), "code", "2??",),
            Matcher::new(MatchOp::Equal, "code", "2??")
        );

        // Test anchoring behavior - should NOT match full string only
        let matcher = Matcher::new(
            MatchOp::NotRe(Matcher::try_parse_re("abc.*").unwrap()),
            "code",
            "abc.*",
        );
        assert!(!matcher.is_match("abc123"));
        assert!(matcher.is_match("xabc123")); // Does not match at start, so NotRe returns true

        let matcher = Matcher::new(
            MatchOp::NotRe(Matcher::try_parse_re(".*xyz$").unwrap()),
            "code",
            ".*xyz",
        );
        assert!(!matcher.is_match("123xyz"));
        assert!(matcher.is_match("123xyzx")); // Does not match at end, so NotRe returns true
    }

    #[test]
    fn test_matchers_equality() {
        assert_eq!(
            Matchers::empty()
                .append(Matcher::new(MatchOp::Equal, "name1", "val1"))
                .append(Matcher::new(MatchOp::Equal, "name2", "val2")),
            Matchers::empty()
                .append(Matcher::new(MatchOp::Equal, "name1", "val1"))
                .append(Matcher::new(MatchOp::Equal, "name2", "val2"))
        );

        assert_ne!(
            Matchers::empty().append(Matcher::new(MatchOp::Equal, "name1", "val1")),
            Matchers::empty().append(Matcher::new(MatchOp::Equal, "name2", "val2"))
        );

        assert_ne!(
            Matchers::empty().append(Matcher::new(MatchOp::Equal, "name1", "val1")),
            Matchers::empty().append(Matcher::new(MatchOp::NotEqual, "name1", "val1"))
        );

        assert_eq!(
            Matchers::empty()
                .append(Matcher::new(MatchOp::Equal, "name1", "val1"))
                .append(Matcher::new(MatchOp::NotEqual, "name2", "val2"))
                .append(Matcher::new(
                    MatchOp::Re(Regex::new("\\d+").unwrap()),
                    "name2",
                    "\\d+"
                ))
                .append(Matcher::new(
                    MatchOp::NotRe(Regex::new("\\d+").unwrap()),
                    "name2",
                    "\\d+"
                )),
            Matchers::empty()
                .append(Matcher::new(MatchOp::Equal, "name1", "val1"))
                .append(Matcher::new(MatchOp::NotEqual, "name2", "val2"))
                .append(Matcher::new(
                    MatchOp::Re(Regex::new("\\d+").unwrap()),
                    "name2",
                    "\\d+"
                ))
                .append(Matcher::new(
                    MatchOp::NotRe(Regex::new("\\d+").unwrap()),
                    "name2",
                    "\\d+"
                ))
        );
    }

    #[test]
    fn test_find_matchers() {
        let matchers = Matchers::empty()
            .append(Matcher::new(MatchOp::Equal, "foo", "bar"))
            .append(Matcher::new(MatchOp::NotEqual, "foo", "bar"))
            .append(Matcher::new_matcher(T_EQL_REGEX, "foo".into(), "bar".into()).unwrap())
            .append(Matcher::new_matcher(T_NEQ_REGEX, "foo".into(), "bar".into()).unwrap())
            .append(Matcher::new(MatchOp::Equal, "FOO", "bar"))
            .append(Matcher::new(MatchOp::NotEqual, "bar", "bar"));

        let ms = matchers.find_matchers("foo");
        assert_eq!(4, ms.len());
    }

    #[test]
    fn test_convert_re() {
        assert_eq!(try_escape_for_repeat_re("abc{}"), r"abc\{}");
        assert_eq!(try_escape_for_repeat_re("abc{def}"), r"abc\{def}");
        assert_eq!(try_escape_for_repeat_re("abc{def"), r"abc\{def");
        assert_eq!(try_escape_for_repeat_re("abc{1}"), "abc{1}");
        assert_eq!(try_escape_for_repeat_re("abc{1,}"), "abc{1,}");
        assert_eq!(try_escape_for_repeat_re("abc{1,2}"), "abc{1,2}");
        assert_eq!(try_escape_for_repeat_re("abc{,2}"), r"abc\{,2}");
        assert_eq!(try_escape_for_repeat_re("abc{{1,2}}"), r"abc\{{1,2}}");
        assert_eq!(try_escape_for_repeat_re(r"abc\{abc"), r"abc\{abc");
        assert_eq!(try_escape_for_repeat_re("abc{1a}"), r"abc\{1a}");
        assert_eq!(try_escape_for_repeat_re("abc{1,a}"), r"abc\{1,a}");
        assert_eq!(try_escape_for_repeat_re("abc{1,2a}"), r"abc\{1,2a}");
        assert_eq!(try_escape_for_repeat_re("abc{1,2,3}"), r"abc\{1,2,3}");
        assert_eq!(try_escape_for_repeat_re("abc{1,,2}"), r"abc\{1,,2}");
    }
}
