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

# Run the ETC workload.
DIST_SCRIPT_DIR=$(dirname $(readlink -f "$0"))
RUN_OPTIONS="-r"

$DIST_SCRIPT_DIR/../basic/cli.sh -c $CONFIG -w prepare-etc $RUN_OPTIONS
$DIST_SCRIPT_DIR/cli.sh -c $CONFIG -w etc-r50 $RUN_OPTIONS
$DIST_SCRIPT_DIR/cli.sh -c $CONFIG -w etc-r95 $RUN_OPTIONS
