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

//! Label matchers and Well-known label names used by Prometheus components.

mod matcher;
pub use matcher::{MatchOp, Matcher, Matchers};
use std::collections::HashSet;

/// "__name__"
pub const METRIC_NAME: &str = "__name__";
/// "alertname"
pub const ALERT_NAME: &str = "alertname";
/// "le"
pub const BUCKET_LABEL: &str = "le";
/// "instance"
pub const INSTANCE_NAME: &str = "instance";

pub type Label = String;
/// Unordered set for a group of labels.
pub type Labels = Vec<Label>;

pub fn new_labels(ls: Vec<&str>) -> Labels {
    ls.iter().map(|s| s.to_string()).collect()
}

pub fn is_labels_joint(ls1: &Labels, ls2: &Labels) -> bool {
    let s1: HashSet<&String> = ls1.iter().collect();
    let s2: HashSet<&String> = ls2.iter().collect();

    !s1.is_disjoint(&s2)
}

pub fn intersect_labels(ls1: &Labels, ls2: &Labels) -> Labels {
    let s1: HashSet<&String> = ls1.iter().collect();
    let s2: HashSet<&String> = ls2.iter().collect();

    s1.intersection(&s2).map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_labels_joint() {
        let cases = vec![
            (vec!["foo"], vec!["bar"], false),
            (vec!["foo"], vec!["foo", "bar"], true),
            (vec!["foo"], vec!["foo"], true),
        ];

        for (lb1, lb2, is) in cases {
            let lb1 = new_labels(lb1);
            let lb2 = new_labels(lb2);
            assert_eq!(is, is_labels_joint(&lb1, &lb2), "{:?} and {:?}", lb1, lb2)
        }
    }

    #[test]
    fn test_intersect_labels() {
        let cases = vec![
            (vec!["foo"], vec!["bar"], vec![]),
            (vec!["foo"], vec!["foo", "bar"], vec!["foo"]),
            (vec!["foo"], vec!["foo"], vec!["foo"]),
            (vec!["foo", "bar"], vec!["bar", "foo"], vec!["foo", "bar"]),
        ];

        for (lb1, lb2, common) in cases {
            let lb1 = new_labels(lb1);
            let lb2 = new_labels(lb2);
            let intersection: HashSet<_> = intersect_labels(&lb1, &lb2).into_iter().collect();
            let expect: HashSet<_> = common.iter().map(|s| s.to_string()).collect();
            assert_eq!(
                expect, intersection,
                "{:?} intersect {:?} does not eq {:?}",
                lb1, lb2, common
            )
        }
    }
}
