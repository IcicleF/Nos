#!/bin/bash

# Figure 8: Slowdown.
# Estimated run time: 

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig8"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

MAXNODES=$(cat $ROOT/scripts/MAXNODES)

CONFIG="c62"
SYSTEMS=("nos" "bsl-cocytus" "bsl-split" "bsl-repl" "bsl-dynbackup" "bsl-split-dyn" "bsl-repl-handoff")

$ROOT/scripts/utils/build.sh --release

# Run experiments.
for system in "${SYSTEMS[@]}"; do
    SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $CONFIG -e $system -r"
    CLIENT_CMD="NOS_CONFIG=$CONFIG $ROOT/scripts/dist-cli/run-ycsb.sh d slowdown"
    rm -rf $DATA_SRC/*-*

    echo "Running config=$CONFIG system=$system value_size=$value ..."
    run_once "$SERVER_CMD" "$CLIENT_CMD"

    # Place the output directory in the correct place.
    OUTPUT_DIR=$(ls $DATA_SRC | grep ycsb | head -n 1)
    if [[ -z $OUTPUT_DIR ]]; then
        echo "Error: No output directory found for the GET workload."
        exit 1
    fi
    mv $DATA_SRC/$OUTPUT_DIR $DATA_DST/$system
    sleep 1
done

mv $DATA_DST/bsl-dynbackup $DATA_DST/bsl-cocytus-plus
mv $DATA_DST/bsl-split-dyn $DATA_DST/bsl-split-plus
mv $DATA_DST/bsl-repl-handoff $DATA_DST/bsl-repl-plus
