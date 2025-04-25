#!/bin/bash

# Figure 4: Microbenchmark, 100% GET/PUT Latency-throughput curves.
# Estimated run time: ???

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig4"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

MAXNODES=$(cat $ROOT/scripts/MAXNODES)

CONFIG="c42"
VALUE="64"

GET_SYSTEMS=("nos" "bsl-split" "bsl-split-lb")
GET_MAXLOAD=(120 30 20)
GET_STEP=(10 5 5)
PUT_SYSTEMS=("nos" "bsl-cocytus" "bsl-pq" "bsl-split" "bsl-repl")
PUT_MAXLOAD=(30 6 6 20 30)
PUT_STEP=(5 2 2 5 5)

$ROOT/scripts/utils/build.sh --release

# Run experiments.
for i in "${!GET_SYSTEMS[@]}"; do
    system=${GET_SYSTEMS[$i]}
    maxload=${GET_MAXLOAD[$i]}
    step=${GET_STEP[$i]}

    SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $CONFIG -e $system -r"
    run_once "$SERVER_CMD" "NOS_CONFIG=$CONFIG $ROOT/scripts/dist-cli/run-prepare.sh $VALUE" nokill

    for LOAD in "1" $(seq $step $step $maxload); do
        rm -rf $DATA_SRC/*-*
        run_once "$SERVER_CMD" "NOS_CONFIG=$CONFIG $ROOT/scripts/dist-cli/run-open.sh $VALUE ${LOAD}m r" nostart+nokill
        
        mkdir -p $DATA_DST/get/$system
        OUTPUT_DIR=$(ls $DATA_SRC | grep readonly | head -n 1)
        if [[ -z $OUTPUT_DIR ]]; then
            echo "Error: No output directory found for the GET workload."
            exit 1
        fi
        mv $DATA_SRC/$OUTPUT_DIR/dump-node0.csv $DATA_DST/get/$system/$LOAD.csv
    done

    $ROOT/scripts/utils/kill.sh
    sleep 1
done

for i in "${!PUT_SYSTEMS[@]}"; do
    system=${PUT_SYSTEMS[$i]}
    maxload=${PUT_MAXLOAD[$i]}
    step=${PUT_STEP[$i]}

    SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $CONFIG -e $system -r"
    run_once "$SERVER_CMD" "NOS_CONFIG=$CONFIG $ROOT/scripts/dist-cli/run-prepare.sh $VALUE" nokill

    for LOAD in "1" $(seq $step $step $maxload); do
        rm -rf $DATA_SRC/*-*
        run_once "$SERVER_CMD" "NOS_CONFIG=$CONFIG $ROOT/scripts/dist-cli/run-open.sh $VALUE ${LOAD}m w" nostart+nokill
        
        mkdir -p $DATA_DST/put/$system
        OUTPUT_DIR=$(ls $DATA_SRC | grep readonly | head -n 1)
        if [[ -z $OUTPUT_DIR ]]; then
            echo "Error: No output directory found for the PUT workload."
            exit 1
        fi

        mv $DATA_SRC/$OUTPUT_DIR/dump-node0.csv $DATA_DST/put/$system/$LOAD.csv
    done

    $ROOT/scripts/utils/kill.sh
    sleep 1
done
