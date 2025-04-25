#!/bin/bash

# Figure 9: Twemcache GET.
# Figure 10: Twemcache PUT.
# Estimated run time: ~20 minutes

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig9"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

CONFIG="c42"
WORKLOADS=("cluster27")
SYSTEMS=("nos" "bsl-cocytus" "bsl-split")

declare -A MAX_LOADS
MAX_LOADS["cluster27-nos"]=80
MAX_LOADS["cluster27-bsl-cocytus"]=30
MAX_LOADS["cluster27-bsl-split"]=20

$ROOT/scripts/utils/build.sh --release

# Run experiments.
for workload in "${WORKLOADS[@]}"; do
    for system in "${SYSTEMS[@]}"; do
        maxload=${MAX_LOADS["$workload-$system"]}
        if [[ -z $maxload ]]; then
            echo "Error: No maxload found for workload $workload and system $system."
            exit 1
        fi

        echo "Running workload $workload with system $system and maxload $maxload."

        SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $CONFIG -e $system -r"
        run_once "$SERVER_CMD" "TRACEFILE=$workload $ROOT/scripts/basic/cli.sh -r -c $CONFIG -w twemcache-prepare" nokill

        for LOAD in "1" $(seq 10 10 $maxload) $((($maxload % 10)) && echo $maxload); do
            rm -rf $DATA_SRC/*-*
            run_once "$SERVER_CMD" "TRACEFILE=$workload NOS_CONFIG=$CONFIG $ROOT/scripts/dist-cli/run-open-trace.sh ${LOAD}m" nostart+nokill
            
            mkdir -p $DATA_DST/$workload/$system
            OUTPUT_DIR=$(ls $DATA_SRC | grep twemcache | head -n 1)
            if [[ -z $OUTPUT_DIR ]]; then
                echo "Error: No output directory found for $workload $system load=$LOAD."
                exit 1
            fi
            mv $DATA_SRC/$OUTPUT_DIR/dump-node0.csv $DATA_DST/$workload/$system/$LOAD.csv
        done

        $ROOT/scripts/utils/kill.sh
        sleep 1
    done
    sleep 1
done
