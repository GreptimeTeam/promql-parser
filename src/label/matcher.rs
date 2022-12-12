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

// Possible MatchTypes.
#[derive(Debug)]
pub enum MatchType {
    MatchEqual,
    MatchNotEqual,
    MatchRegexp,
    MatchNotRegexp,
}

// Matcher models the matching of a label.
#[derive(Debug)]
pub struct Matcher {
    typ: MatchType,
    name: String,
    value: String,
    // FIXME: Regex Matcher
    // re *FastRegexMatcher
}

impl Matcher {
    pub fn new(t: MatchType, n: &str, v: &str) -> Self {
        Self {
            typ: t,
            name: n.into(),
            value: n.into(),
        }
    }

    // Matches returns whether the matcher matches the given string value.
    pub fn matches(&self, s: &str) -> bool {
        match self.typ {
            MatchType::MatchEqual => self.value.eq(s),
            MatchType::MatchNotEqual => self.value.ne(s),
            MatchType::MatchRegexp => todo!(),
            MatchType::MatchNotRegexp => todo!(),
        }
    }
}
