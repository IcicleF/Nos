#!/bin/bash

CONFIG="${NOS_CONFIG}"
WORKLOAD="abd"
SLOWDOWN_ON_D="0"

# If provided with a workload, use it.
if [[ $# -ge 1 ]]; then
    WORKLOAD=$1
fi

# If slowdown is enabled, set the flag.
if [[ $# -ge 2 ]]; then
    # Slowdown if the second argument is "1", "true", "yes", "y", "on", "slowdown", case-insensitive.
    # No-slowdown if the second argument is "0", "false", "no", "n", "off", case-insensitive.
    if [[ "$2" =~ ^(1|true|yes|y|on|slowdown)$ ]]; then
        SLOWDOWN_ON_D="1"
        echo "Slowdown enabled for YCSB-D."
    else 
        if [[ "$2" =~ ^(0|false|no|n|off)$ ]]; then
            SLOWDOWN_ON_D="0"
        else
            SLOWDOWN_ON_D="0"
            echo "Error: Invalid argument for slowdown, default to no slowdown."
        fi
    fi
fi

# Run the YCSB workloads.
DIST_SCRIPT_DIR=$(dirname $(readlink -f "$0"))
RUN_OPTIONS="-r"

$DIST_SCRIPT_DIR/run-prepare.sh 1k
$DIST_SCRIPT_DIR/../utils/kill.sh cli

if [[ "$WORKLOAD" == *"a"* || "$WORKLOAD" == *"A"* ]]; then
    $DIST_SCRIPT_DIR/cli.sh -c $CONFIG -w ycsb-a $RUN_OPTIONS
    $DIST_SCRIPT_DIR/../utils/kill.sh cli
fi

if [[ "$WORKLOAD" == *"b"* || "$WORKLOAD" == *"B"* ]]; then
    $DIST_SCRIPT_DIR/cli.sh -c $CONFIG -w ycsb-b $RUN_OPTIONS
    $DIST_SCRIPT_DIR/../utils/kill.sh cli
fi

if [[ "$WORKLOAD" == *"d"* || "$WORKLOAD" == *"D"* ]]; then
    if [[ "$SLOWDOWN_ON_D" == "1" ]]; then
        # Start the special client to slow down the nodes in tmux.
        SLOWDOWN_SESSION="nos-slowdown"
        tmux kill-session -t $SLOWDOWN_SESSION >> /dev/null 2>&1
        tmux new-session -s $SLOWDOWN_SESSION -n slowdown-cli -d
        tmux send-keys -t $SLOWDOWN_SESSION "$DIST_SCRIPT_DIR/../special-cli/slowdown.sh $CONFIG" C-m
    fi

    sleep 1
    $DIST_SCRIPT_DIR/cli.sh -c $CONFIG -w ycsb-d $RUN_OPTIONS

    if [[ "$SLOWDOWN_ON_D" == "1" ]]; then
        # Kill the slowdown session.
        tmux send-keys -t $SLOWDOWN_SESSION C-c
        tmux send-keys -t $SLOWDOWN_SESSION C-c
        sleep 1
        tmux kill-session -t $SLOWDOWN_SESSION
    fi
    
    $DIST_SCRIPT_DIR/../utils/kill.sh cli
fi
