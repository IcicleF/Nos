#!/bin/bash
# Benchmark latency under a given load.

if [[ $# -lt 2 ]]; then
    echo "Usage: $0 <VALUE_SIZE> <OFFERED_LOAD> [rwn]"
    echo
    echo "Available value sizes: [64, 128, 256, 1k, 4k, test]"
    echo "                                              ^^^^ 1M * 1kB value, for quick testing"
    echo
    echo "OFFERED_LOAD is the overall load, i.e., thpt_per_client * num_clients."
    echo "Specify 'r' or 'R' to run read benchmarks, 'w' or 'W' to run write benchmarks (default: rw)."
    echo "Specify 'n' or 'N' to skip dataset preparation."
    exit 1
fi

CONFIG=${NOS_CONFIG}
SIZE=$1
LOAD=$2
RW=${3:-"rw"}

# Error if config is not set.
if [[ -z "$CONFIG" ]]; then
    echo "Error: Must specify config by \`NOS_CONFIG\`."
    exit 1
fi

DIST_SCRIPT_DIR=$(dirname $(readlink -f "$0"))
WORKSPACE_ROOT=$(python3 $DIST_SCRIPT_DIR/../utils/find-root.py)

echo "Compiling ..."
$DIST_SCRIPT_DIR/../utils/build.sh --release --quiet
echo "Compilation finished."
echo

# Get the request rate.
# - Client number = nodes in the cluster.
NUM_CLIENTS=$($WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/release/cluster2ip -n -c $WORKSPACE_ROOT/config/$CONFIG.toml)
# - Thread number = defined in the workload.
NUM_THREADS=$(cat $WORKSPACE_ROOT/config/$CONFIG.toml | grep "rpc =" | awk '{print $3}')

RATE=$(python3 $DIST_SCRIPT_DIR/../utils/compute-rate.py $LOAD $NUM_CLIENTS $NUM_THREADS)
echo "average interval between requests: $RATE ns"
echo

# Prepare the workload before running the actual clients.
COMMON_OPTS="--release --config $CONFIG"

# Run the benchmark once.
# Signature: run_once <WORKLOAD>
run_once() {
    local WORKLOAD=$1

    # We need to run two (sets of) clients separately.
    # 1. Run in a tmux session, start open-loop clients at all nodes except the first one.
    # 2. Run in the current session, start open-loop clients at the first node.

    # Step 1. Launch remote clients.
    local ANOTHER_CLIENT_CMD="$DIST_SCRIPT_DIR/cli.sh $COMMON_OPTS --workload $WORKLOAD --selection 1.. --policy open:$RATE"
    tmux new-session -d -s "openloop-remote-cli" "$ANOTHER_CLIENT_CMD"

    # Step 2. Launch the local client.
    $DIST_SCRIPT_DIR/cli.sh $COMMON_OPTS --workload $WORKLOAD --selection 0 --policy open-measured:$RATE

    # Step 3. Sleep for a grace period, and kill remote clients.
    sleep 5
    tmux kill-session -t openloop-remote-cli 2>/dev/null
}

# Run read and write microbenchmarks.
if [[ $RW == *"r"* || $RW == *"R"* ]]; then
    run_once "readonly-$SIZE"
fi

if [[ $RW == *"w"* || $RW == *"W"* ]]; then
    run_once "updateonly-$SIZE"
fi
