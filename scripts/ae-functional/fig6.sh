#!/bin/bash

# Figure 6: Microbenchmark, sensitivity.
# Estimated run time: ~11 minutes

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig6"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

MAXNODES=$(cat $ROOT/scripts/MAXNODES)

P_VALUES=("2" "3")
SYSTEMS=("nos" "bsl-cocytus" "bsl-split")

$ROOT/scripts/utils/build.sh --release

# Run experiments.
for p in "${P_VALUES[@]}"; do
    config="c4$p"
    for system in "${SYSTEMS[@]}"; do
        SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $config -e $system -r"
        CLIENT_CMD="NOS_CONFIG=$config $ROOT/scripts/dist-cli/run-ycsb.sh a"
    
        rm -rf $DATA_SRC/*-*
        echo "Running config=$config system=$system ..."
        run_once "$SERVER_CMD" "$CLIENT_CMD"

        # Place the output directory in the correct place.
        mkdir -p $DATA_DST/$config

        OUTPUT_DIR=$(ls $DATA_SRC | grep ycsb | head -n 1)
        if [[ -z $OUTPUT_DIR ]]; then
            echo "Error: No output directory found for $config $system."
            exit 1
        fi
        mv $DATA_SRC/$OUTPUT_DIR $DATA_DST/$config/$system
        sleep 1
    done
done
