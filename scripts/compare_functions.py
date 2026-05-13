#!/usr/bin/env python3
"""
Script to compare Prometheus Go functions.go with Rust functions.rs
Ensures Rust functions are complete and consistent with Go version.
"""

import re
import sys
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass


@dataclass
class FunctionDef:
    name: str
    arg_types: List[str]
    variadic: int
    return_type: str
    experimental: bool

    def __str__(self):
        exp_flag = " [EXP]" if self.experimental else ""
        return f"{self.name}: args={self.arg_types}, variadic={self.variadic}, return={self.return_type}{exp_flag}"


def parse_go_functions(content: str) -> Dict[str, FunctionDef]:
    """Parse Prometheus Go functions.go to extract function definitions."""
    functions = {}

    # Find all function entry points
    entries = []
    pattern = r'"([^"]+)":\s*\{'
    for match in re.finditer(pattern, content):
        entries.append((match.start(), match.end(), match.group(1)))

    # Extract each function block
    for i, (start, end, name) in enumerate(entries):
        block_start = end
        # Find the closing brace for this block
        brace_count = 1
        block_end = block_start
        while brace_count > 0 and block_end < len(content):
            block_end += 1
            if content[block_end] == "{":
                brace_count += 1
            elif content[block_end] == "}":
                brace_count -= 1

        block_content = content[block_start:block_end]

        # Parse Name field
        name_match = re.search(r'Name:\s*"([^"]+)"', block_content)
        if not name_match:
            continue

        # Parse ArgTypes field
        arg_types = []
        arg_types_match = re.search(
            r"ArgTypes:\s*\[\]ValueType\{(.*?)\}", block_content, re.DOTALL
        )
        if arg_types_match:
            arg_types_str = arg_types_match.group(1).strip()
            for arg in re.findall(r"ValueType(\w+)", arg_types_str):
                arg_types.append(arg)

        # Parse Variadic field
        variadic = 0
        variadic_match = re.search(r"Variadic:\s*(-?\d+)", block_content)
        if variadic_match:
            variadic = int(variadic_match.group(1))

        # Parse ReturnType field
        return_type = ""
        return_match = re.search(r"ReturnType:\s*([^,\n}]+)", block_content)
        if return_match:
            return_type_clean = re.sub(r"ValueType", "", return_match.group(1)).strip()
            return_type = return_type_clean

        # Parse Experimental field
        experimental = False
        experimental_match = re.search(r"Experimental:\s*(true|false)", block_content)
        if experimental_match:
            experimental = experimental_match.group(1).lower() == "true"

        functions[name] = FunctionDef(
            name=name,
            arg_types=arg_types,
            variadic=variadic,
            return_type=return_type,
            experimental=experimental,
        )

    return functions


def parse_rust_functions(content: str) -> Dict[str, FunctionDef]:
    """Parse Rust functions.rs to extract function definitions."""
    functions = {}

    # Pattern to match function! macro calls
    # Example:
    # function!("abs", vec![ValueType::Vector], 0, ValueType::Vector, false),
    # function!("days_in_month", vec![ValueType::Vector], 1, ValueType::Vector, false),
    # function!("label_join", vec![ValueType::Vector, ValueType::String, ValueType::String, ValueType::String], -1, ValueType::Vector, false),
    # function!("double_exponential_smoothing", vec![ValueType::Matrix, ValueType::Scalar, ValueType::Scalar], 0, ValueType::Vector, true),
    pattern = r'function!\(\s*"([^"]+)"\s*,\s*vec!\[(.*?)\]\s*,\s*(-?\d+)\s*,\s*ValueType::(\w+)\s*,\s*(true|false)\s*\),'

    for match in re.finditer(pattern, content, re.DOTALL):
        name = match.group(1)
        arg_types_str = match.group(2).strip()
        variadic = int(match.group(3))
        return_type = match.group(4)
        experimental = match.group(5).lower() == "true"

        # Parse arg types
        arg_types = []
        if arg_types_str:
            for arg in re.findall(r"ValueType::(\w+)", arg_types_str):
                arg_types.append(arg)

        functions[name] = FunctionDef(
            name=name,
            arg_types=arg_types,
            variadic=variadic,
            return_type=return_type,
            experimental=experimental,
        )

    return functions


def normalize_type(type_str: str) -> str:
    """Normalize type names for comparison."""
    # Map Go types to Rust types
    type_mapping = {
        "String": "String",
        "None": "None",
    }
    return type_mapping.get(type_str, type_str)


def compare_functions(go_func: FunctionDef, rust_func: FunctionDef) -> List[str]:
    """Compare two function definitions and return list of differences."""
    differences = []

    # Compare arg types
    go_args = [normalize_type(t) for t in go_func.arg_types]
    rust_args = [normalize_type(t) for t in rust_func.arg_types]

    if go_args != rust_args:
        differences.append(f"  Arg types differ: Go={go_args}, Rust={rust_args}")

    # Compare variadic
    if go_func.variadic != rust_func.variadic:
        differences.append(
            f"  Variadic differs: Go={go_func.variadic}, Rust={rust_func.variadic}"
        )

    # Compare return type
    go_return = normalize_type(go_func.return_type)
    rust_return = normalize_type(rust_func.return_type)
    if go_return != rust_return:
        differences.append(f"  Return type differs: Go={go_return}, Rust={rust_return}")

    # Compare experimental flag
    if go_func.experimental != rust_func.experimental:
        differences.append(
            f"  Experimental flag differs: Go={go_func.experimental}, Rust={rust_func.experimental}"
        )

    return differences


def main():
    import subprocess

    # Fetch Prometheus Go functions.go from GitHub
    go_url = "https://raw.githubusercontent.com/prometheus/prometheus/main/promql/parser/functions.go"
    print(f"Fetching Prometheus functions.go from {go_url}...")

    try:
        result = subprocess.run(
            ["curl", "-s", go_url], capture_output=True, text=True, check=True
        )
        go_content = result.stdout
    except Exception as e:
        print(f"Error fetching Go file: {e}")
        sys.exit(1)

    # Read Rust functions.rs
    rust_file = "src/parser/function.rs"
    print(f"Reading Rust functions from {rust_file}...")

    try:
        with open(rust_file, "r") as f:
            rust_content = f.read()
    except Exception as e:
        print(f"Error reading Rust file: {e}")
        sys.exit(1)

    # Parse both files
    go_functions = parse_go_functions(go_content)
    rust_functions = parse_rust_functions(rust_content)

    print(f"\nParsed {len(go_functions)} functions from Go")
    print(f"Parsed {len(rust_functions)} functions from Rust\n")

    # Find missing functions in Rust
    missing_in_rust = set(go_functions.keys()) - set(rust_functions.keys())

    # Find extra functions in Rust
    extra_in_rust = set(rust_functions.keys()) - set(go_functions.keys())

    # Find differences in common functions
    common_functions = set(go_functions.keys()) & set(rust_functions.keys())
    differences = {}

    for func_name in sorted(common_functions):
        go_func = go_functions[func_name]
        rust_func = rust_functions[func_name]

        diff = compare_functions(go_func, rust_func)
        if diff:
            differences[func_name] = (go_func, rust_func, diff)

    # Print results
    print("=" * 80)
    print("COMPARISON RESULTS")
    print("=" * 80)

    if missing_in_rust:
        print(f"\n❌ {len(missing_in_rust)} function(s) MISSING in Rust:")
        for func in sorted(missing_in_rust):
            print(f"  - {func}")

    if extra_in_rust:
        print(f"\n⚠️  {len(extra_in_rust)} function(s) EXTRA in Rust (not in Go):")
        for func in sorted(extra_in_rust):
            print(f"  - {func}")

    if differences:
        print(f"\n🔍 {len(differences)} function(s) have DIFFERENCES:")
        for func_name in sorted(differences.keys()):
            go_func, rust_func, diff = differences[func_name]
            print(f"\n  {func_name}:")
            print(f"    Go version:   {go_func}")
            print(f"    Rust version: {rust_func}")
            for d in diff:
                print(f"    {d}")

    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)

    total_go = len(go_functions)
    total_rust = len(rust_functions)
    total_common = len(common_functions)
    total_differences = len(differences)

    print(f"Go functions:     {total_go}")
    print(f"Rust functions:   {total_rust}")
    print(f"Common functions: {total_common}")
    print(f"Missing in Rust:  {len(missing_in_rust)}")
    print(f"Extra in Rust:    {len(extra_in_rust)}")
    print(f"Differences:      {total_differences}")

    if not missing_in_rust and not differences:
        print("\n✅ All functions are COMPLETE and CONSISTENT!")
        sys.exit(0)
    else:
        print("\n❌ Issues found - please review and fix")
        sys.exit(1)


if __name__ == "__main__":
    main()
