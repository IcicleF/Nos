#!/bin/bash

# Figure 11: Recovery.
# Estimated run time: ~6 minutes

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig11"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

mkdir -p $DATA_DST/a
mkdir -p $DATA_DST/b

CONFIG="c62-recovery"
PROFILE="degraded2"
VALUES=("64")
SYSTEMS=("nos" "bsl-cocytus" "bsl-split" "bsl-repl")

for value in "${VALUES[@]}"; do
    for system in "${SYSTEMS[@]}"; do
        SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $CONFIG -e $system -r"
        CLIENT_CMD="NOS_CONFIG=\"$CONFIG\" $ROOT/scripts/dist-cli/run-repair.sh $value $PROFILE-$value 1"

        echo "Running $system $value node-repair ..."
        rm -rf $DATA_SRC/*-*
        run_once "$SERVER_CMD" "$CLIENT_CMD"

        # Place the output directory in the correct place.
        mkdir -p $DATA_DST/$value

        OUTPUT_DIR=$(ls $DATA_SRC | grep degraded | head -n 1)
        if [[ -z $OUTPUT_DIR ]]; then
            echo "Error: No output directory found for $value $system."
            exit 1
        fi
        mv $DATA_SRC/$OUTPUT_DIR/dump-node0.csv $DATA_DST/$value/$system.csv
        rm -rf $DATA_SRC/$OUTPUT_DIR
        sleep 1
    done
done
