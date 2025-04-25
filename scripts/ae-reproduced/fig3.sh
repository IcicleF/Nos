#!/bin/bash

# Figure 3: Microbenchmark, 100% GET/PUT.
# Figure 12: Memory consumption.
# Estimated run time: ???

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig3"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

MAXNODES=$(cat $ROOT/scripts/MAXNODES)

CONFIGS=("c42" "c62" "c63")
SYSTEMS=("nos" "bsl-cocytus" "bsl-pq" "bsl-split" "bsl-repl")
VALUES=("64" "256" "1k" "4k")

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
            REDO="rw"
        
            # Repeat the experiment until it succeeds.
            while true; do
                CLIENT_CMD="NOS_CONFIG=$config $ROOT/scripts/dist-cli/run-thpt.sh $value $REDO"
                REDO=""

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
                if [[ $(ls $DATA_SRC/$OUTPUT_DIR | wc -l) -ne $MAXNODES ]]; then
                    echo "Warning: experiment failure, retrying."
                    REDO="${REDO}r"
                    exit 1
                else
                    mv $DATA_SRC/$OUTPUT_DIR $DATA_DST/$config/$system/$value/r
                fi

                OUTPUT_DIR=$(ls $DATA_SRC | grep updateonly | head -n 1)
                if [[ -z $OUTPUT_DIR ]]; then
                    echo "Error: No output directory found for the PUT workload."
                    exit 1
                fi
                if [[ $(ls $DATA_SRC/$OUTPUT_DIR | wc -l) -ne $MAXNODES ]]; then
                    echo "Warning: experiment failure, retrying."
                    REDO="${REDO}w"
                    exit 1
                else
                    mv $DATA_SRC/$OUTPUT_DIR $DATA_DST/$config/$system/$value/w
                fi

                # If no redo is needed, break the loop.
                if [[ -z $REDO ]]; then
                    break
                fi
            done
        done
    done
done
