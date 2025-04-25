#!/bin/bash

# Figure 3: Microbenchmark, 100% GET/PUT.
# Figure 12: Memory consumption.
# Estimated run time: ~8 minutes

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig3"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

MEMORY_DST="$ROOT/ae-data/fig12"
mkdir -p $MEMORY_DST
rm -rf $MEMORY_DST/*
touch $MEMORY_DST/memory.txt

MAXNODES=$(cat $ROOT/scripts/MAXNODES)

CONFIGS=("c42")
SYSTEMS=("nos" "bsl-cocytus" "bsl-pq" "bsl-split" "bsl-repl")
VALUES=("64")

$ROOT/scripts/utils/build.sh --release

# Run experiments.
for config in "${CONFIGS[@]}"; do
    for system in "${SYSTEMS[@]}"; do
        # P+Q does not support p != 2.
        if [[ $system == "bsl-pq" && $config == "c63" ]]; then
            continue
        fi
        # For Repl, k is irrelevant.
        if [[ $system == "bsl-repl" && $config == "c62" ]]; then
            continue
        fi

        for value in "${VALUES[@]}"; do
            SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $config -e $system -r"
            CLIENT_CMD="NOS_CONFIG=$config $ROOT/scripts/dist-cli/run-thpt.sh $value rw $system"
        
            rm -rf $DATA_SRC/*-*
            echo "Running config=$config system=$system value_size=$value ..."
            run_once "$SERVER_CMD" "$CLIENT_CMD"

            # Place the output directory in the correct place.
            mkdir -p $DATA_DST/$config/$system/$value

            OUTPUT_DIR=$(ls $DATA_SRC | grep readonly | head -n 1)
            if [[ -z $OUTPUT_DIR ]]; then
                echo "Error: No output directory found for the GET workload."
                exit 1
            fi
            mv $DATA_SRC/$OUTPUT_DIR $DATA_DST/$config/$system/$value/r

            OUTPUT_DIR=$(ls $DATA_SRC | grep updateonly | head -n 1)
            if [[ -z $OUTPUT_DIR ]]; then
                echo "Error: No output directory found for the PUT workload."
                exit 1
            fi
            mv $DATA_SRC/$OUTPUT_DIR $DATA_DST/$config/$system/$value/w

            # Retrieve memory usage.
            if [[ -f $DATA_SRC/memory-$system-$value ]]; then
                MEMORY_USAGE=$(cat $DATA_SRC/memory-$system-$value)
                echo "$config $system $value $MEMORY_USAGE" >> $MEMORY_DST/memory.txt
                rm $DATA_SRC/memory-$system-$value
            fi
        done
    done
done
