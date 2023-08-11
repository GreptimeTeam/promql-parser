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

//! Internal utilities for parser.

pub mod duration;
pub mod number;
mod visitor;

pub use duration::{display_duration, parse_duration};
pub use number::parse_str_radix;
pub use visitor::{walk_expr, ExprVisitor};

pub(crate) fn join_vector<T: std::fmt::Display>(v: &[T], sep: &str, sort: bool) -> String {
    let mut vs = v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    if sort {
        vs.sort();
    }
    vs.join(sep)
}
