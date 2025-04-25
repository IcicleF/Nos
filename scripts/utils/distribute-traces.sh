#!/bin/bash

SCRIPT_DIR=$(dirname $(readlink -f "$0"))
SCRIPTS_DIR=$(dirname $SCRIPT_DIR)

MAXNODES=$(cat $SCRIPTS_DIR/MAXNODES)
MAXNODE=$((MAXNODES - 1))

if [[ $(hostname) != *"node0"* ]]; then
    echo "It seems that you are not on node0."
    echo "- If you are using our CloudLab profile, please go to node0."
    echo "- Otherwise, this script is unlikely to work, and you need to manually copy the trace files."
    exit 1
fi

TRACE_DIR="$HOME/nos-traces"
DATASETS=("04" "12" "27" "31")

pdsh -w ssh:node[0-$MAXNODE] "mkdir -p $TRACE_DIR"

for i in $(seq 0 $MAXNODE); do
    for ds in "${DATASETS[@]}"; do
        if [[ ! -f "/dataset/cluster$ds-$i" ]]; then
            echo "Dataset integrity error: /dataset/cluster$ds-$i does not exist."
            exit 1
        fi
    done
done

for i in $(seq 0 $MAXNODE); do
    echo "Copying traces to node$i..."
    for ds in "${DATASETS[@]}"; do
        scp /dataset/cluster$ds-$i node$i:$TRACE_DIR/cluster$ds.tr
    done
    echo
done

echo "Done."
