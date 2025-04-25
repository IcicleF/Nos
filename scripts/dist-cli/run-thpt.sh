#!/bin/bash
# Benchmark throughput.

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <VALUE_SIZE> [rw] [<EXEC>]"
    echo
    echo "Available value sizes: [64, 128, 256, 1k, 4k]"
    echo "Specify 'r' or 'R' to run read benchmarks, 'w' or 'W' to run write benchmarks (default: rw)."
    echo "If <EXEC> is specified, fetch the memory usage by the way."
    exit 1
fi

CONFIG=${NOS_CONFIG}
SIZE=$1
RW=${2:-"rw"}
EXEC=${3:-""}

# Error if config is not set.
if [[ -z "$CONFIG" ]]; then
    echo "Error: Must specify config by \`NOS_CONFIG\`."
    exit 1
fi

# Run read and update microbenchmarks.
DIST_SCRIPT_DIR=$(dirname $(readlink -f "$0"))
RUN_OPTIONS="--release --policy closed"

$DIST_SCRIPT_DIR/run-prepare.sh $SIZE
$DIST_SCRIPT_DIR/../utils/kill.sh cli

if [[ -n "$EXEC" ]]; then
    $DIST_SCRIPT_DIR/../utils/get-memory.sh $EXEC > $DIST_SCRIPT_DIR/../../data/uncollected/memory-$EXEC-$SIZE
fi

if [[ $RW == *"r"* || $RW == *"R"* ]]; then
    $DIST_SCRIPT_DIR/cli.sh --config $CONFIG --workload readonly-$SIZE $RUN_OPTIONS --port 31851
    $DIST_SCRIPT_DIR/../utils/kill.sh cli
fi

if [[ $RW == *"w"* || $RW == *"W"* ]]; then
    $DIST_SCRIPT_DIR/cli.sh --config $CONFIG --workload updateonly-zipf-$SIZE $RUN_OPTIONS --port 31851
    $DIST_SCRIPT_DIR/../utils/kill.sh cli
fi
