#!/bin/bash

CONFIG="${NOS_CONFIG}"

# If config is not set, use the first argument as the config name.
if [[ -z "$CONFIG" ]]; then
    if [[ $# -ne 1 ]]; then
        echo "Error: Must specify a config name when \`NOS_CONFIG\` is not set."
        echo "Usage: $0 <cluster_config_name>"
        exit 1
    fi
    CONFIG=$1
fi

# Build.
SCRIPT_DIR=$(dirname $(readlink -f "$0"))
WORKSPACE_ROOT=$(python3 $SCRIPT_DIR/../utils/find-root.py)
$SCRIPT_DIR/../utils/build.sh --package nos-cli --quiet

if [[ $? -ne 0 ]]; then
    echo "compilation failed, abort."
    exit 1
fi

# The two nodes to slow down.
A=0
B=16

# Slowdown nodes 0 & 1.
echo "Waiting 10s before slowing down..."
sleep 10
$WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/debug/nos-slowdown-cli $WORKSPACE_ROOT/config/$CONFIG.toml +$A +$B

echo
echo "Waiting 25s before recovering..."
sleep 25
$WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/debug/nos-slowdown-cli $WORKSPACE_ROOT/config/$CONFIG.toml -$A -$B

echo
echo "Done."
