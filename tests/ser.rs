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
            serde_json::to_value(ast).expect("Failed to serialize"),
            serde_json::json!($json)
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
        "type": "vectorSelector",
        "startOrEnd": null,
        "timestamp": null
    });

    assert_json_ser_eq!(
        "prometheus_tsdb_wal_writes_failed_total{label != \"nice\"}",

        {
            "matchers": [
                {
                    "name": "label",
                    "type": "!=",
                    "value": "nice"
                }
            ],
            "name": "prometheus_tsdb_wal_writes_failed_total",
            "offset": 0,
            "type": "vectorSelector",
            "startOrEnd": null,
            "timestamp": null
        }
    );

    assert_json_ser_eq!(
        "prometheus_tsdb_wal_writes_failed_total{label =~ \"nice\"}",

    {
        "matchers": [
            {
                "name": "label",
                "type": "=~",
                "value": "nice"
            }
        ],
        "name": "prometheus_tsdb_wal_writes_failed_total",
        "offset": 0,
        "type": "vectorSelector",
        "startOrEnd": null,
        "timestamp": null
    });

    assert_json_ser_eq!(
        "prometheus_tsdb_wal_writes_failed_total{label !~ \"nice\"}",

    {
        "matchers": [
            {
                "name": "label",
                "type": "!~",
                "value": "nice"
            }
        ],
        "name": "prometheus_tsdb_wal_writes_failed_total",
        "offset": 0,
        "type": "vectorSelector",
        "startOrEnd": null,
        "timestamp": null
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
        "prometheus_tsdb_wal_writes_failed_total offset -2s @ end()",

    {
        "matchers": [],
        "name": "prometheus_tsdb_wal_writes_failed_total",
        "offset": -2000,
        "startOrEnd": "end",
        "timestamp": null,
        "type": "vectorSelector"
    });

    assert_json_ser_eq!(
        "prometheus_tsdb_wal_writes_failed_total @ 1000",

    {
        "matchers": [],
        "name": "prometheus_tsdb_wal_writes_failed_total",
        "offset": 0,
        "startOrEnd": null,
        "timestamp": 1000000,
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
                "type": "matrixSelector",
                "startOrEnd": null,
                "timestamp": null
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

    assert_json_ser_eq!("\"yes\"",
    {
        "type": "stringLiteral",
        "val": "yes"
    }
    );

    assert_json_ser_eq!("1",
    {
        "type": "numberLiteral",
        "val": "1"
    }
    );

    assert_json_ser_eq!("1+1",
    {
        "bool": false,
        "lhs": {
            "type": "numberLiteral",
            "val": "1"
        },
        "op": "+",
        "matching": null,
        "rhs": {
            "type": "numberLiteral",
            "val": "1"
        },
        "type": "binaryExpr"
    }
    );

    assert_json_ser_eq!("- process_cpu_seconds_total",
        {
            "expr": {
                "matchers": [],
                "name": "process_cpu_seconds_total",
                "offset": 0,
                "type": "vectorSelector",
                "startOrEnd": null,
                "timestamp": null
            },
            "op": "-",
            "type": "unaryExpr"
        }
    );

    assert_json_ser_eq!(r#"1 - ((node_memory_MemAvailable_bytes{job="node"} or (node_memory_Buffers_bytes{job="node"} + node_memory_Cached_bytes{job="node"} + node_memory_MemFree_bytes{job="node"} + node_memory_Slab_bytes{job="node"}) ) / node_memory_MemTotal_bytes{job="node"})"#,
        {
            "bool": false,
            "lhs": {
                "type": "numberLiteral",
                "val": "1"
            },
            "op": "-",
            "matching": null,
            "rhs": {
                "expr": {
                    "bool": false,
                    "lhs": {
                        "expr": {
                            "bool": false,
                            "lhs": {
                                "matchers": [
                                    {
                                        "name": "job",
                                        "type": "=",
                                        "value": "node"
                                    }
                                ],
                                "name": "node_memory_MemAvailable_bytes",
                                "offset": 0,
                                "type": "vectorSelector",
                                "startOrEnd": null,
                                "timestamp": null
                            },
                            "matching": {
                                "card": "many-to-many",
                                "include": [],
                                "labels": [],
                                "on": false
                            },
                            "op": "or",
                            "rhs": {
                                "expr": {
                                    "bool": false,
                                    "lhs": {
                                        "bool": false,
                                        "lhs": {
                                            "bool": false,
                                            "lhs": {
                                                "matchers": [
                                                    {
                                                        "name": "job",
                                                        "type": "=",
                                                        "value": "node"
                                                    }
                                                ],
                                                "name": "node_memory_Buffers_bytes",
                                                "offset": 0,
                                                "type": "vectorSelector",
                                                "startOrEnd": null,
                                                "timestamp": null
                                            },
                                            "matching": null,
                                            "op": "+",
                                            "rhs": {
                                                "matchers": [
                                                    {
                                                        "name": "job",
                                                        "type": "=",
                                                        "value": "node"
                                                    }
                                                ],
                                                "name": "node_memory_Cached_bytes",
                                                "offset": 0,
                                                "type": "vectorSelector",
                                                "startOrEnd": null,
                                                "timestamp": null
                                            },
                                            "type": "binaryExpr"
                                        },
                                        "matching": null,
                                        "op": "+",
                                        "rhs": {
                                            "matchers": [
                                                {
                                                    "name": "job",
                                                    "type": "=",
                                                    "value": "node"
                                                }
                                            ],
                                            "name": "node_memory_MemFree_bytes",
                                            "offset": 0,
                                            "type": "vectorSelector",
                                            "startOrEnd": null,
                                            "timestamp": null
                                        },
                                        "type": "binaryExpr"
                                    },
                                    "matching": null,
                                    "op": "+",
                                    "rhs": {
                                        "matchers": [
                                            {
                                                "name": "job",
                                                "type": "=",
                                                "value": "node"
                                            }
                                        ],
                                        "name": "node_memory_Slab_bytes",
                                        "offset": 0,
                                        "type": "vectorSelector",
                                        "startOrEnd": null,
                                        "timestamp": null
                                    },
                                    "type": "binaryExpr"
                                },
                                "type": "parenExpr"
                            },
                            "type": "binaryExpr"
                        },
                        "type": "parenExpr"
                    },
                    "matching": null,
                    "op": "/",
                    "rhs": {
                        "matchers": [
                            {
                                "name": "job",
                                "type": "=",
                                "value": "node"
                            }
                        ],
                        "name": "node_memory_MemTotal_bytes",
                        "offset": 0,
                        "type": "vectorSelector",
                        "startOrEnd": null,
                        "timestamp": null
                    },
                    "type": "binaryExpr"
                },
                "type": "parenExpr"
            },
            "type": "binaryExpr"
        }
    );

    assert_json_ser_eq!("foo * on(branch) bar ",
    {
        "bool": false,
        "lhs": {
            "matchers": [],
            "name": "foo",
            "offset": 0,
            "type": "vectorSelector",
            "startOrEnd": null,
            "timestamp": null
        },
        "matching": {
            "card": "one-to-one",
            "include": [],
            "labels": [
            "branch"
            ],
            "on": true
        },
        "op": "*",
        "rhs": {
            "matchers": [],
            "name": "bar",
            "offset": 0,
            "type": "vectorSelector",
            "startOrEnd": null,
            "timestamp": null
        },
        "type": "binaryExpr"
    });

    assert_json_ser_eq!("foo * ignoring(branch) bar ",
        {
            "bool": false,
            "lhs": {
                "matchers": [],
                "name": "foo",
                "offset": 0,
                "type": "vectorSelector",
                "startOrEnd": null,
                "timestamp": null
            },
            "matching": {
                "card": "one-to-one",
                "include": [],
                "labels": [
                    "branch"
                ],
                "on": false
            },
            "op": "*",
            "rhs": {
                "matchers": [],
                "name": "bar",
                "offset": 0,
                "type": "vectorSelector",
                "startOrEnd": null,
                "timestamp": null
            },
            "type": "binaryExpr"
        }
    );

    assert_json_ser_eq!("min_over_time( rate(http_requests_total[5m])[30m:1m] )",
        {
            "args": [
                {
                    "expr": {
                        "args": [
                            {
                                "matchers": [],
                                "name": "http_requests_total",
                                "offset": 0,
                                "range": 300000,
                                "type": "matrixSelector",
                                "startOrEnd": null,
                                "timestamp": null
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
                    },
                    "offset": 0,
                    "range": 1800000,
                    "step": 60000,
                    "type": "subquery",
                    "startOrEnd": null,
                    "timestamp": null
                }
            ],
            "func": {
                "argTypes": [
                    "matrix"
                ],
                "name": "min_over_time",
                "returnType": "vector",
                "variadic": 0
            },
            "type": "call"

    }
    );

    assert_json_ser_eq!("sum(rate(http_requests_total[5m]))",
    {
        "expr": {
        "args": [
            {
            "matchers": [],
            "name": "http_requests_total",
            "offset": 0,
            "range": 300000,
            "type": "matrixSelector",
            "startOrEnd": null,
            "timestamp": null
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
        },
        "grouping": [],
        "op": "sum",
        "param": null,
        "type": "aggregation",
        "without": false
    });

    assert_json_ser_eq!("sum by(host) (rate(http_requests_total[5m]))",
    {
        "expr": {
        "args": [
            {
            "matchers": [],
            "name": "http_requests_total",
            "offset": 0,
            "range": 300000,
            "type": "matrixSelector",
            "startOrEnd": null,
            "timestamp": null
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
        },
        "grouping": ["host"],
        "op": "sum",
        "param": null,
        "type": "aggregation",
        "without": false
    });

    assert_json_ser_eq!("sum without(host) (rate(http_requests_total[5m]))",
    {
        "expr": {
        "args": [
            {
            "matchers": [],
            "name": "http_requests_total",
            "offset": 0,
            "range": 300000,
            "type": "matrixSelector",
            "startOrEnd": null,
            "timestamp": null
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
        },
        "grouping": ["host"],
        "op": "sum",
        "param": null,
        "type": "aggregation",
        "without": true
    });

    assert_json_ser_eq!("min by (name,namespace,cluster) (certmanager_certificate_expiration_timestamp_seconds)-time() <= 15d",
    {
        "bool":false,
        "lhs":{
            "bool":false,
            "lhs":{
                "expr":{
                "matchers":[],
                "name":"certmanager_certificate_expiration_timestamp_seconds",
                "offset":0,
                "startOrEnd":null,
                "timestamp":null,
                "type":"vectorSelector"
                },
                "grouping":[
                "name",
                "namespace",
                "cluster"
                ],
                "op":"min",
                "param":null,
                "type":"aggregation",
                "without":false
            },
            "matching":null,
            "op":"-",
            "rhs":{
                "args":[

                ],
                "func":{
                "argTypes":[

                ],
                "name":"time",
                "returnType":"scalar",
                "variadic":0
                },
                "type":"call"
            },
            "type":"binaryExpr"
        },
        "matching":null,
        "op":"<=",
        "rhs":{
            "type":"numberLiteral",
            "val":"1296000"
        },
        "type":"binaryExpr"
    });
}
