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

pub type Label = String;
pub type Labels = HashSet<Label>;
