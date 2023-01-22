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

/// to put it simple, if diff < 0.000000001, they are equal
/// TODO: better solution
pub fn f64_equals(x: f64, y: f64) -> bool {
    x - y < 0.000_000_001
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_eqlaus() {
        assert!(f64_equals(0.1, 0.05 + 0.05));
        assert!(f64_equals(0.01, 0.005 + 0.005));
        assert!(f64_equals(0.001, 0.0005 + 0.0005));
        assert!(f64_equals(0.15 + 0.15 + 0.15, 0.1 + 0.1 + 0.25));
    }
}
