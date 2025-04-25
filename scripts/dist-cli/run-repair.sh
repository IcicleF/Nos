#!/bin/bash
# Benchmark throughput.

if [[ $# -lt 3 ]]; then
    echo "Usage: $0 <VALUE_SIZE> <PROFILE> <RECOVERY_TARGET>"
    echo
    echo "Available value sizes: [64, 128, 256, 1k, 4k]"
    exit 1
fi

CONFIG=${NOS_CONFIG}
SIZE=$1
PROFILE=$2
TARGET=$3

# Error if config is not set.
if [[ -z "$CONFIG" ]]; then
    echo "Error: Must specify config by \`NOS_CONFIG\`."
    exit 1
fi

# Run read and update microbenchmarks.
DIST_SCRIPT_DIR=$(dirname $(readlink -f "$0"))

$DIST_SCRIPT_DIR/run-prepare.sh $SIZE
$DIST_SCRIPT_DIR/../utils/kill.sh cli

SCRIPTS_DIR=$(dirname $DIST_SCRIPT_DIR)

$DIST_SCRIPT_DIR/../basic/cli.sh -c $CONFIG -w $PROFILE -r --policy recover:$TARGET --prepare off --dump
