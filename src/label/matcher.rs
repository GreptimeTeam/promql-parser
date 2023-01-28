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

use std::collections::HashMap;

use crate::label::METRIC_NAME;
use crate::parser::token::{TokenType, T_EQL, T_EQL_REGEX, T_NEQ, T_NEQ_REGEX};
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

// Matcher models the matching of a label.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn new_eq_name(value: String) -> Self {
        Self {
            op: MatchOp::Equal,
            name: METRIC_NAME.into(),
            value,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    // matches returns whether the matcher matches the given string value.
    pub fn is_match(&self, s: &str) -> bool {
        match &self.op {
            MatchOp::Equal => self.value.eq(s),
            MatchOp::NotEqual => self.value.ne(s),
            MatchOp::Re(r) => r.is_match(s),
            MatchOp::NotRe(r) => !r.is_match(s),
        }
    }

    pub fn new_matcher(id: TokenType, name: String, value: String) -> Result<Matcher, String> {
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
            _ => Err(format!("invalid match op {}", id)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Matchers {
    pub matchers: Vec<Matcher>,
}

impl Matchers {
    pub fn empty() -> Self {
        Self { matchers: vec![] }
    }

    pub fn new(matchers: Vec<Matcher>) -> Self {
        Self { matchers }
    }

    pub fn append(mut self, matcher: Matcher) -> Self {
        self.matchers.push(matcher);
        self
    }
}

impl PartialEq for Matchers {
    fn eq(&self, other: &Self) -> bool {
        if self.matchers.len() != other.matchers.len() {
            return false;
        }

        let selfs: HashMap<_, _> = self.matchers.iter().map(|m| (m.name.clone(), m)).collect();
        let others: HashMap<_, _> = other.matchers.iter().map(|m| (m.name.clone(), m)).collect();

        if selfs.len() != others.len() {
            return false;
        }

        for (name, s_matcher) in selfs {
            match others.get(&name) {
                Some(o_matcher) if s_matcher.eq(o_matcher) => continue,
                _ => return false,
            };
        }
        true
    }
}

impl Eq for Matchers {}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_matcher_equality() {
        let eq_matcher1 = Matcher::new(MatchOp::Equal, String::from("code"), String::from("200"));
        let eq_matcher2 = Matcher::new(MatchOp::Equal, String::from("code"), String::from("200"));
        assert_eq!(eq_matcher1, eq_matcher2);

        let ne_matcher1 =
            Matcher::new(MatchOp::NotEqual, String::from("code"), String::from("200"));
        let ne_matcher2 =
            Matcher::new(MatchOp::NotEqual, String::from("code"), String::from("200"));
        assert_eq!(ne_matcher1, ne_matcher2);

        let re_matcher1 = Matcher::new(
            MatchOp::Re(Regex::new("2??").unwrap()),
            String::from("code"),
            String::from("2??"),
        );
        let re_matcher2 = Matcher::new(
            MatchOp::Re(Regex::new("2??").unwrap()),
            String::from("code"),
            String::from("2??"),
        );
        assert_eq!(re_matcher1, re_matcher2);

        let not_re_matcher1 = Matcher::new(
            MatchOp::NotRe(Regex::new("2??").unwrap()),
            String::from("code"),
            String::from("2??"),
        );
        let not_re_matcher2 = Matcher::new(
            MatchOp::NotRe(Regex::new("2??").unwrap()),
            String::from("code"),
            String::from("2??"),
        );
        assert_eq!(not_re_matcher1, not_re_matcher2);
    }
}
