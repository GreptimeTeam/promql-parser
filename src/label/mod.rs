mod matcher;

pub use matcher::{MatchType, Matcher};

// Well-known label names used by Prometheus components.
pub const METRIC_NAME: &'static str = "__name__";
pub const ALERT_NAME: &'static str = "alertname";
pub const BUCKET_LABEL: &'static str = "le";
pub const INSTANCE_NAME: &'static str = "instance";

/// Label is a key/value pair of strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    name: String,
    value: String,
}

impl Label {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

// Labels is a sorted set of labels. Order has to be guaranteed upon
// instantiation.
pub type Labels = Vec<Label>;

/// sort labels by name in alphabetical order, case insensitive.
pub fn sort_labels(mut labels: Labels) -> Labels {
    labels.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_labels() {
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
        let mut labels = vec![rust.clone(), go.clone(), clojure.clone()];

        labels = sort_labels(labels);
        assert_eq!(labels, vec![clojure, go, rust]);
    }
}
