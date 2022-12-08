mod matcher;

pub use matcher::{MatchType, Matcher};

// Well-known label names used by Prometheus components.
const METRIC_NAME: &'static str = "__name__";
const ALERT_NAME: &'static str = "alertname";
const BUCKET_LABEL: &'static str = "le";
const INSTANCE_NAME: &'static str = "instance";

// Label is a key/value pair of strings.
pub struct Label {
    name: String,
    value: String,
}

// Labels is a sorted set of labels. Order has to be guaranteed upon
// instantiation.
pub type Labels = Vec<Label>;
