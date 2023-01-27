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

mod matcher;

pub use matcher::{MatchOp, Matcher, Matchers};
use std::collections::HashSet;

// Well-known label names used by Prometheus components.
pub const METRIC_NAME: &str = "__name__";
pub const ALERT_NAME: &str = "alertname";
pub const BUCKET_LABEL: &str = "le";
pub const INSTANCE_NAME: &str = "instance";

/// Label is a key/value pair of strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    pub value: String,
}

impl Label {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

// Labels is a set of labels.
pub struct Labels {
    pub labels: Vec<Label>,
}

impl Labels {
    pub fn empty() -> Self {
        Self { labels: vec![] }
    }

    pub fn new(labels: Vec<Label>) -> Self {
        Self { labels }
    }

    pub fn append(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// match_labels returns a subset of Labels that matches/does not match with the provided label names based on the 'on' boolean.
    /// If on is set to true, it returns the subset of labels that match with the provided label names and its inverse when 'on' is set to false.
    pub fn match_labels(&self, on: bool, names: Vec<String>) -> Vec<Label> {
        let set: HashSet<String> = names.into_iter().collect();
        let mut result = vec![];
        for label in &self.labels {
            let contains = set.contains(&label.name);
            // if on is false, then METRIC_NAME CAN NOT be included in the result
            if on == contains && (on || !label.name.eq_ignore_ascii_case(METRIC_NAME)) {
                result.push(label.clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: more test cases needed in prometheus/model/labels/matcher_test.go
    #[test]
    fn test_match_labels() {
        let rust = Label {
            name: "Rust".into(),
            value: "rust".into(),
        };
        let go = Label {
            name: "go".into(),
            value: "go".into(),
        };
        let clojure = Label {
            name: "Clojure".into(),
            value: "Clojure".into(),
        };
        let labels = Labels::new(vec![rust.clone(), go.clone(), clojure.clone()]);

        let matched_labels = labels.match_labels(true, vec!["go".into()]);
        assert_eq!(1, matched_labels.len());
        assert_eq!(go, matched_labels[0]);

        let matched_labels = labels.match_labels(false, vec!["go".into()]);
        assert_eq!(2, matched_labels.len());
    }
}
