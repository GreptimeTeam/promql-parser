// Copyright 2022 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::parser::Token;
use crate::parser::{T_EQL, T_EQL_REGEX, T_NEQ, T_NEQ_REGEX};
use regex::Regex;

#[derive(Debug)]
pub enum MatchOp {
    Equal,
    NotEqual,
    Re(Regex),
    NotRe(Regex),
}

// Matcher models the matching of a label.
#[derive(Debug)]
pub struct Matcher {
    op: MatchOp,
    name: String,
    value: String,
}

impl Matcher {
    pub fn new(op: MatchOp, name: String, value: String) -> Self {
        Self { op, name, value }
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
}

#[derive(Debug)]
pub struct Matchers {
    matchers: Vec<Matcher>,
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

pub fn new_matcher(token: Token, name: String, value: String) -> Result<Matcher, String> {
    match token.id() {
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
        _ => Err(format!("invalid match op {}", token.val())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_ne() {
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
    fn test_re() {
        let value = "api/v1/.*".to_string();
        let re = Regex::new(&value).unwrap();
        let op = MatchOp::Re(re);
        let matcher = Matcher::new(op, "name".into(), value);
        assert!(matcher.is_match("api/v1/query"));
        assert!(matcher.is_match("api/v1/range_query"));
        assert!(!matcher.is_match("api/v2"));
    }
}
