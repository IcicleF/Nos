#!/bin/bash

# Figure 9: Twemcache GET.
# Figure 10: Twemcache PUT.
# Estimated run time: ???

ROOT=$(python3 $(dirname $(readlink -f "$0"))/../utils/find-root.py)
source $ROOT/scripts/utils/run-once.fn.sh

DATA_SRC="$ROOT/data/uncollected"
DATA_DST="$ROOT/ae-data/fig9"
mkdir -p $DATA_DST
rm -rf $DATA_DST/*

CONFIG="c42"
WORKLOADS=("cluster04" "cluster12" "cluster27" "cluster31")
SYSTEMS=("nos" "bsl-cocytus" "bsl-pq" "bsl-split" "bsl-repl")

declare -A MAX_LOADS
MAX_LOADS["cluster04-nos"]=80
MAX_LOADS["cluster04-bsl-cocytus"]=44
MAX_LOADS["cluster04-bsl-pq"]=45
MAX_LOADS["cluster04-bsl-split"]=25
MAX_LOADS["cluster04-bsl-repl"]=80

MAX_LOADS["cluster12-nos"]=33
MAX_LOADS["cluster12-bsl-cocytus"]=8
MAX_LOADS["cluster12-bsl-pq"]=8
MAX_LOADS["cluster12-bsl-split"]=20
MAX_LOADS["cluster12-bsl-repl"]=34

MAX_LOADS["cluster27-nos"]=80
MAX_LOADS["cluster27-bsl-cocytus"]=32
MAX_LOADS["cluster27-bsl-pq"]=32
MAX_LOADS["cluster27-bsl-split"]=25
MAX_LOADS["cluster27-bsl-repl"]=80

MAX_LOADS["cluster31-nos"]=35
MAX_LOADS["cluster31-bsl-cocytus"]=8
MAX_LOADS["cluster31-bsl-pq"]=8
MAX_LOADS["cluster31-bsl-split"]=18
MAX_LOADS["cluster31-bsl-repl"]=30

# Run experiments.
for workload in "${WORKLOADS[@]}"; do
    for system in "${SYSTEMS[@]}"; do
        maxload=${MAX_LOADS["$workload-$system"]}
        if [[ -z $maxload ]]; then
            echo "Error: No maxload found for workload $workload and system $system."
            exit 1
        fi

        SERVER_CMD="$ROOT/scripts/basic/svr.sh -c $CONFIG -e $system -r"
        run_once "WORKLOAD=$workload $ROOT/scripts/basic/cli.sh -r -c $CONFIG -w twemcache-prepare" nokill

        for LOAD in "1" $(seq 10 10 $maxload) $((($maxload % 10)) && echo $maxload); do
            rm -rf $DATA_SRC/*-*
            run_once "$SERVER_CMD" "WORKLOAD=$workload NOS_CONFIG=$CONFIG $ROOT/scripts/dist-cli/run-open-trace.sh ${LOAD}m" nostart+nokill
            
            mkdir -p $DATA_DST/$workload/$system
            OUTPUT_DIR=$(ls $DATA_SRC | grep readonly | head -n 1)
            if [[ -z $OUTPUT_DIR ]]; then
                echo "Error: No output directory found for $workload $system load=$LOAD."
                exit 1
            fi
            mv $DATA_SRC/$OUTPUT_DIR/dump-node0.csv $DATA_DST/$workload/$system/$LOAD.csv
        done
        sleep 1
    done
    sleep 1
done
