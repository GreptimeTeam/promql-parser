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

#![cfg(feature = "ser")]

use promql_parser::parser::parse;

macro_rules! assert_json_ser_eq {
    ($promql: literal, $json: tt) => {
        let ast = parse($promql).expect("Failed to parse");
        assert_eq!(
            serde_json::json!($json),
            serde_json::to_value(ast).expect("Failed to serialize")
        );
    };
}

#[test]
fn test_serialize() {
    assert_json_ser_eq!(
        "prometheus_tsdb_wal_writes_failed_total",

    {
        "matchers": [],
        "name": "prometheus_tsdb_wal_writes_failed_total",
        "offset": 0,
        "type": "vectorSelector"
    });

    assert_json_ser_eq!(
        "prometheus_tsdb_wal_writes_failed_total offset 2s @ start()",

    {
        "matchers": [],
        "name": "prometheus_tsdb_wal_writes_failed_total",
        "offset": 2000,
        "startOrEnd": "start",
        "timestamp": null,
        "type": "vectorSelector"
    });

    assert_json_ser_eq!(
            "rate(prometheus_tsdb_wal_writes_failed_total{instance = \"localhost:9090\"}[1m])",

    {
        "args": [
            {
                "matchers": [
                    {
                        "name": "instance",
                        "type": "=",
                        "value": "localhost:9090"
                    }
                ],
                "name": "prometheus_tsdb_wal_writes_failed_total",
                "offset": 0,
                "range": 60000,
                "type": "matrixSelector"
            }
        ],
        "func": {
            "argTypes": [
                "matrix"
            ],
            "name": "rate",
            "returnType": "vector",
            "variadic": 0
        },
        "type": "call"
    }

    );
}
