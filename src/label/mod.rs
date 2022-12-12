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
