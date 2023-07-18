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
use std::fmt;

pub type Label = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    pub labels: Vec<Label>,
}

impl Labels {
    pub fn append(mut self, l: Label) -> Self {
        self.labels.push(l);
        self
    }

    pub fn new(ls: Vec<&str>) -> Self {
        let labels = ls.iter().map(|s| s.to_string()).collect();
        Self { labels }
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn is_joint(&self, ls: &Labels) -> bool {
        let s1: HashSet<&String> = self.labels.iter().collect();
        let s2: HashSet<&String> = ls.labels.iter().collect();

        !s1.is_disjoint(&s2)
    }

    pub fn intersect(&self, ls: &Labels) -> Labels {
        let s1: HashSet<&String> = self.labels.iter().collect();
        let s2: HashSet<&String> = ls.labels.iter().collect();
        let labels = s1.intersection(&s2).map(|s| s.to_string()).collect();

        Self { labels }
    }
}

impl fmt::Display for Labels {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let labels = self.labels.join(", ");
        write!(f, "({labels})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_string() {
        let cases = vec![
            (vec!["foo"], "(foo)"),
            (vec!["foo", "bar"], "(foo, bar)"),
            (vec!["foo", "foo", "bar"], "(foo, foo, bar)"),
        ];

        for (ls, expect) in cases {
            let lb = Labels::new(ls);
            assert_eq!(expect, lb.to_string())
        }
    }

    #[test]
    fn test_is_joint() {
        let cases = vec![
            (vec!["foo"], vec!["bar"], false),
            (vec!["foo"], vec!["foo", "bar"], true),
            (vec!["foo"], vec!["foo"], true),
        ];

        for (lb1, lb2, is) in cases {
            let lb1 = Labels::new(lb1);
            let lb2 = Labels::new(lb2);
            assert_eq!(is, lb1.is_joint(&lb2), "{:?} and {:?}", lb1, lb2)
        }
    }

    #[test]
    fn test_intersect() {
        let cases = vec![
            (vec!["foo"], vec!["bar"], vec![]),
            (vec!["foo"], vec!["foo", "bar"], vec!["foo"]),
            (vec!["foo"], vec!["foo"], vec!["foo"]),
            (vec!["foo", "bar"], vec!["bar", "foo"], vec!["foo", "bar"]),
        ];

        for (lb1, lb2, common) in cases {
            let lb1 = Labels::new(lb1);
            let lb2 = Labels::new(lb2);
            let intersection: HashSet<_> = lb1.intersect(&lb2).labels.into_iter().collect();
            let expect: HashSet<_> = common.iter().map(|s| s.to_string()).collect();
            assert_eq!(
                expect, intersection,
                "{:?} intersect {:?} does not eq {:?}",
                lb1, lb2, common
            )
        }
    }
}
