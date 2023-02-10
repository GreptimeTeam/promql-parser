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

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::label::METRIC_NAME;
use crate::parser::token::{TokenId, T_EQL, T_EQL_REGEX, T_NEQ, T_NEQ_REGEX};
use regex::Regex;

#[derive(Debug, Clone)]
pub enum MatchOp {
    Equal,
    NotEqual,
    Re(Regex),
    NotRe(Regex),
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

// Matcher models the matching of a label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Matcher {
    pub op: MatchOp,
    pub name: String,
    pub value: String,
}

impl Matcher {
    pub fn new(op: MatchOp, name: String, value: String) -> Self {
        Self { op, name, value }
    }

    /// build a matcher instance with default metric name and Equal operation
    pub fn new_eq_metric_matcher(value: String) -> Self {
        Self {
            op: MatchOp::Equal,
            name: METRIC_NAME.into(),
            value,
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

    pub fn new_matcher(id: TokenId, name: String, value: String) -> Result<Matcher, String> {
        match id {
            T_EQL => Ok(Matcher::new(MatchOp::Equal, name, value)),
            T_NEQ => Ok(Matcher::new(MatchOp::NotEqual, name, value)),
            T_EQL_REGEX => {
                let re = Regex::new(&value).map_err(|_| format!("illegal regex for {}", &value))?;
                Ok(Matcher::new(MatchOp::Re(re), name, value))
            }
            T_NEQ_REGEX => {
                let re = Regex::new(&value).map_err(|_| format!("illegal regex for {}", &value))?;
                Ok(Matcher::new(MatchOp::NotRe(re), name, value))
            }
            _ => Err(format!("invalid match op {id}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matchers {
    pub matchers: HashSet<Matcher>,
}

impl Matchers {
    pub fn empty() -> Self {
        Self {
            matchers: HashSet::new(),
        }
    }

    pub fn one(matcher: Matcher) -> Self {
        let matchers = HashSet::from([matcher]);
        Self { matchers }
    }

    pub fn new(matchers: HashSet<Matcher>) -> Self {
        Self { matchers }
    }

    pub fn append(mut self, matcher: Matcher) -> Self {
        self.matchers.insert(matcher);
        self
    }

    /// Vector selectors must either specify a name or at least one label
    /// matcher that does not match the empty string.
    ///
    /// The following expression is illegal:
    /// {job=~".*"} # Bad!
    pub fn is_empty_matchers(&self) -> bool {
        self.matchers.is_empty() || self.matchers.iter().all(|m| m.is_match(""))
    }

    /// find all the matchers whose name equals the specified name.
    pub fn find_matchers(&self, name: &str) -> Vec<&String> {
        self.matchers
            .iter()
            .filter(|m| m.name.eq_ignore_ascii_case(name))
            .map(|m| &m.value)
            .collect()
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
            Err(format!("invalid match op {}", token::T_ADD))
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
            hash(Matcher::new(MatchOp::Equal, "name".into(), "value".into())),
            hash(Matcher::new(MatchOp::Equal, "name".into(), "value".into())),
        );

        assert_eq!(
            hash(Matcher::new(
                MatchOp::NotEqual,
                "name".into(),
                "value".into()
            )),
            hash(Matcher::new(
                MatchOp::NotEqual,
                "name".into(),
                "value".into()
            )),
        );

        assert_eq!(
            hash(Matcher::new(
                MatchOp::Re(Regex::new("\\s+").unwrap()),
                "name".into(),
                "\\s+".into()
            )),
            hash(Matcher::new(
                MatchOp::Re(Regex::new("\\s+").unwrap()),
                "name".into(),
                "\\s+".into()
            )),
        );

        assert_eq!(
            hash(Matcher::new(
                MatchOp::NotRe(Regex::new("\\s+").unwrap()),
                "name".into(),
                "\\s+".into()
            )),
            hash(Matcher::new(
                MatchOp::NotRe(Regex::new("\\s+").unwrap()),
                "name".into(),
                "\\s+".into()
            )),
        );

        assert_ne!(
            hash(Matcher::new(MatchOp::Equal, "name".into(), "value".into())),
            hash(Matcher::new(
                MatchOp::NotEqual,
                "name".into(),
                "value".into()
            )),
        );

        assert_ne!(
            hash(Matcher::new(
                MatchOp::Re(Regex::new("\\s+").unwrap()),
                "name".into(),
                "\\s+".into()
            )),
            hash(Matcher::new(
                MatchOp::NotRe(Regex::new("\\s+").unwrap()),
                "name".into(),
                "\\s+".into()
            )),
        );
    }

    #[test]
    fn test_matcher_eq_ne() {
        let op = MatchOp::Equal;
        let matcher = Matcher::new(op, "name".into(), "up".into());
        assert!(matcher.is_match("up"));
        assert!(!matcher.is_match("down"));

        let op = MatchOp::NotEqual;
        let matcher = Matcher::new(op, "name".into(), "up".into());
        assert!(matcher.is_match("foo"));
        assert!(matcher.is_match("bar"));
        assert!(!matcher.is_match("up"));
    }

    #[test]
    fn test_matcher_re() {
        let value = "api/v1/.*".to_string();
        let re = Regex::new(&value).unwrap();
        let op = MatchOp::Re(re);
        let matcher = Matcher::new(op, "name".into(), value);
        assert!(matcher.is_match("api/v1/query"));
        assert!(matcher.is_match("api/v1/range_query"));
        assert!(!matcher.is_match("api/v2"));
    }

    #[test]
    fn test_eq_matcher_equality() {
        assert_eq!(
            Matcher::new(MatchOp::Equal, String::from("code"), String::from("200")),
            Matcher::new(MatchOp::Equal, String::from("code"), String::from("200"))
        );

        assert_ne!(
            Matcher::new(MatchOp::Equal, String::from("code"), String::from("200")),
            Matcher::new(MatchOp::Equal, String::from("code"), String::from("201"))
        );

        assert_ne!(
            Matcher::new(MatchOp::Equal, String::from("code"), String::from("200")),
            Matcher::new(MatchOp::NotEqual, String::from("code"), String::from("200"))
        );
    }

    #[test]
    fn test_ne_matcher_equality() {
        assert_eq!(
            Matcher::new(MatchOp::NotEqual, String::from("code"), String::from("200")),
            Matcher::new(MatchOp::NotEqual, String::from("code"), String::from("200"))
        );

        assert_ne!(
            Matcher::new(MatchOp::NotEqual, String::from("code"), String::from("200")),
            Matcher::new(MatchOp::NotEqual, String::from("code"), String::from("201"))
        );

        assert_ne!(
            Matcher::new(MatchOp::NotEqual, String::from("code"), String::from("200")),
            Matcher::new(MatchOp::Equal, String::from("code"), String::from("200"))
        );
    }

    #[test]
    fn test_re_matcher_equality() {
        assert_eq!(
            Matcher::new(
                MatchOp::Re(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2??"),
            ),
            Matcher::new(
                MatchOp::Re(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2??"),
            )
        );

        assert_ne!(
            Matcher::new(
                MatchOp::Re(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2??"),
            ),
            Matcher::new(
                MatchOp::Re(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2*?"),
            )
        );

        assert_ne!(
            Matcher::new(
                MatchOp::Re(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2??"),
            ),
            Matcher::new(MatchOp::Equal, String::from("code"), String::from("2??"))
        );
    }

    #[test]
    fn test_not_re_matcher_equality() {
        assert_eq!(
            Matcher::new(
                MatchOp::NotRe(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2??"),
            ),
            Matcher::new(
                MatchOp::NotRe(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2??"),
            )
        );

        assert_ne!(
            Matcher::new(
                MatchOp::NotRe(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2??"),
            ),
            Matcher::new(
                MatchOp::NotRe(Regex::new("2?*").unwrap()),
                String::from("code"),
                String::from("2*?"),
            )
        );

        assert_ne!(
            Matcher::new(
                MatchOp::NotRe(Regex::new("2??").unwrap()),
                String::from("code"),
                String::from("2??"),
            ),
            Matcher::new(MatchOp::Equal, String::from("code"), String::from("2??"))
        );
    }

    #[test]
    fn test_matchers_equality() {
        assert_eq!(
            Matchers::empty()
                .append(Matcher::new(MatchOp::Equal, "name1".into(), "val1".into()))
                .append(Matcher::new(MatchOp::Equal, "name1".into(), "val1".into()))
                .append(Matcher::new(MatchOp::Equal, "name2".into(), "val2".into())),
            Matchers::empty()
                .append(Matcher::new(MatchOp::Equal, "name1".into(), "val1".into()))
                .append(Matcher::new(MatchOp::Equal, "name2".into(), "val2".into()))
        );

        assert_ne!(
            Matchers::empty().append(Matcher::new(MatchOp::Equal, "name1".into(), "val1".into())),
            Matchers::empty().append(Matcher::new(MatchOp::Equal, "name2".into(), "val2".into()))
        );

        assert_ne!(
            Matchers::empty().append(Matcher::new(MatchOp::Equal, "name1".into(), "val1".into())),
            Matchers::empty().append(Matcher::new(
                MatchOp::NotEqual,
                "name1".into(),
                "val1".into()
            ))
        );

        assert_eq!(
            Matchers::empty()
                .append(Matcher::new(MatchOp::Equal, "name1".into(), "val1".into()))
                .append(Matcher::new(
                    MatchOp::NotEqual,
                    "name2".into(),
                    "val2".into()
                ))
                .append(Matcher::new(
                    MatchOp::Re(Regex::new("\\d+").unwrap()),
                    "name2".into(),
                    "\\d+".into()
                ))
                .append(Matcher::new(
                    MatchOp::NotRe(Regex::new("\\d+").unwrap()),
                    "name2".into(),
                    "\\d+".into()
                )),
            Matchers::empty()
                .append(Matcher::new(MatchOp::Equal, "name1".into(), "val1".into()))
                .append(Matcher::new(
                    MatchOp::NotEqual,
                    "name2".into(),
                    "val2".into()
                ))
                .append(Matcher::new(
                    MatchOp::Re(Regex::new("\\d+").unwrap()),
                    "name2".into(),
                    "\\d+".into()
                ))
                .append(Matcher::new(
                    MatchOp::NotRe(Regex::new("\\d+").unwrap()),
                    "name2".into(),
                    "\\d+".into()
                ))
        );
    }
}
