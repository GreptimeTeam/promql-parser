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
pub type Labels = HashSet<Label>;
