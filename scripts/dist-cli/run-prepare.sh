#!/bin/bash
# Prepare records in a distributed manner.

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <VALUE_SIZE>"
    echo "Available value sizes: [64, 128, 256, 1k, 4k]"
    exit 1
fi

CONFIG=${NOS_CONFIG}
SIZE=$1

# Error if config is not set.
if [[ -z "$CONFIG" ]]; then
    echo "Error: Must specify config by \`NOS_CONFIG\`."
    exit 1
fi

DIST_SCRIPT_DIR=$(dirname $(readlink -f "$0"))
ROOT=$(dirname $(dirname $DIST_SCRIPT_DIR))
CORE_LIST="18-23,42-47"

$DIST_SCRIPT_DIR/../utils/build.sh --package nos-cli --quiet --release

MAXNODES=$(cat $ROOT/scripts/MAXNODES)
MAXNODE=$(($MAXNODES - 1))
pdsh -w ssh:node[0-$MAXNODE] "RUST_LOG=info $ROOT/target/x86_64-unknown-linux-gnu/release/nos-cli \
    --config $ROOT/config/$CONFIG.toml --workload $ROOT/config/workloads/prepare-$SIZE.toml \
    --policy closed --bind-core $CORE_LIST --prepare \$(hostname | grep -oP '^\D*\K\d+')/$MAXNODES"
sleep 1
