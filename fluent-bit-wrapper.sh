#!/bin/bash

bin/fluent-bit \
    -i dummy -t test1 \
        -p "Dummy = {\"Example1\": 3, \"Example2\": 4, \"Example3\": 5}" \
        -p "Samples = 3" \
    -F wasm -m 'test*' \
        -p "WASM_Path = filter_dp.$wasm_optimization" \
        -p "Function_Name = filter_dp" \
        -p "accessible_paths = filters" \
    -o stdout -m '*' \
    | while IFS= read -r line; do
    echo "$line"  # or process the line as needed
    if [[ "$line" == *"ExitSignal"* ]]; then
        # Gracefully terminate Fluent Bit and this script
        kill -TERM $$
    fi
done
