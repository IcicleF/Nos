#!/bin/bash

# Figure 5: Microbenchmark, YCSB.
# Estimated run time: ~8 minutes

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig5"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

MAXNODES=$(cat $ROOT/scripts/MAXNODES)

CONFIGS=("c42")
SYSTEMS=("nos" "bsl-cocytus" "bsl-split" "bsl-repl")

$ROOT/scripts/utils/build.sh --release

# Run experiments.
for config in "${CONFIGS[@]}"; do
    for system in "${SYSTEMS[@]}"; do
        SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $config -e $system -r"
        CLIENT_CMD="NOS_CONFIG=$config $ROOT/scripts/dist-cli/run-ycsb.sh a"
    
        rm -rf $DATA_SRC/*-*
        echo "Running config=$config system=$system ..."
        run_once "$SERVER_CMD" "$CLIENT_CMD"

        # Place the output directory in the correct place.
        mkdir -p $DATA_DST/$config/$system

        for workload in "a"; do
            OUTPUT_DIR=$(ls $DATA_SRC | grep ycsb-$workload | head -n 1)
            if [[ -z $OUTPUT_DIR ]]; then
                echo "Error: No output directory found for YCSB-$workload workload."
                exit 1
            fi
            mv $DATA_SRC/$OUTPUT_DIR $DATA_DST/$config/$system/$workload
        done
        sleep 1
    done
done
