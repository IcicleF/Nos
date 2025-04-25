#!/bin/bash

BUILD_TYPE="debug"
LOG_LEVEL="info"
CONFIG="${NOS_CONFIG:-lab}"
WORKLOAD="readonly-mwe"
CORE_LIST="18-23,42-47"
POLICY="closed"
SELECTION=""
SMPORT="31850"

VALID_ARGS=$(getopt -o hrc:w:p:b:s:l: --long help,release,config:,workload:,policy:,bind:,selection:,log:,port: -- "$@")
if [[ $? -ne 0 ]]; then
    exit 1;
fi

eval set -- "$VALID_ARGS"
while [ : ]; do
    case $1 in
        -h | --help)
            echo "Distributed client runner script"
            echo
            echo "Usage: cli.sh [OPTIONS]"
            echo
            echo "Options:"
            echo "  -h, --help                   Print this help message and exit."
            echo "  -r, --release                Run release version instead of debug."
            echo "  -c, --config <CONFIG>        Specify config file name (only name, without \`.toml\` suffix). [default: default]"
            echo "                               Config files will be looked up in ./config/ directory."
            echo "                               If \`NOS_CONFIG\` is set, it will be used as the config file name."
            echo "  -w, --workload <WORKLOAD>    Specify workload file name (only name, without \`.toml\` suffix). [default: small]"
            echo "                               Workload files will be looked up in ./config/workloads/ directory."
            echo "  -p, --policy <POLICY>        Specify execution policy. [default: closed]"
            echo "  -b, --bind <CORELIST>        Specify the cores to bind on. [default: 18-23,42-47]"
            echo "  -s, --selection <SELECTION>  Set the node selection policy."
            echo "  -l, --log <LEVEL>            Specify log level. [default: info]"
            echo "      --port <PORT>            Specify rrppcc port. [default: 31850]"
            exit 0
            ;;
        -r | --release)
            BUILD_TYPE="release"
            shift
            ;;
        -c | --config)
            CONFIG=$2
            shift 2
            ;;
        -w | --workload)
            WORKLOAD=$2
            shift 2
            ;;
        -p | --policy)
            POLICY=$2
            shift 2
            ;;
        -b | --bind)
            CORE_LIST=$2
            shift 2
            ;;
        -s | --selection)
            SELECTION=$2
            shift 2
            ;;
        -l | --log)
            LOG_LEVEL=$2
            shift 2
            ;;
        --port)
            SMPORT=$2
            shift 2
            ;;
        --) 
            shift;
            break
            ;;
    esac
done

echo "[distributed client]"
echo "build type:    $BUILD_TYPE"
echo "config:        $CONFIG"
echo "workload:      $WORKLOAD"
echo "policy:        $POLICY"
echo "bind cores:    $CORE_LIST"
echo "log level:     $LOG_LEVEL"

if [ -n "$SELECTION" ]; then
    echo "nodes select:  $SELECTION"
    SELECTION="--selection $SELECTION"
else
    echo "nodes select:  all"
fi

echo

# Build and run.
SCRIPT_DIR=$(dirname $(readlink -f "$0"))
WORKSPACE_ROOT=$(python3 $SCRIPT_DIR/../utils/find-root.py)

BUILD_OPTIONS="--package nos-cli"
if [[ $BUILD_TYPE == "release" ]]; then
    BUILD_OPTIONS="$BUILD_OPTIONS --release"
fi
$SCRIPT_DIR/../utils/build.sh $BUILD_OPTIONS

if [[ $? -ne 0 ]]; then
    echo "compilation failed, abort."
    exit 1
else
    echo "compilation finished."
fi
echo

# Do not dump if policy is open-loop without measurement.
if [[ $POLICY == *"open:"* ]]; then
    echo
    echo "no dump data for open-loop policy."
    echo
    DUMP_OPTION=""
else
    TIMESTAMP=$(date +'%Y%m%d-%H%M%S')
    mkdir -p $WORKSPACE_ROOT/data
    mkdir -p $WORKSPACE_ROOT/data/uncollected/
    mkdir -p $WORKSPACE_ROOT/data/uncollected/$WORKLOAD-$TIMESTAMP/
    DUMP_OPTION="--dump $WORKSPACE_ROOT/data/uncollected/$WORKLOAD-$TIMESTAMP/dump.csv"
fi

SSH_TARGETS=$($WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/$BUILD_TYPE/cluster2ip -n $SELECTION $WORKSPACE_ROOT/config/$CONFIG.toml)
pdsh -w ssh:$SSH_TARGETS "TRACEFILE=\"$TRACEFILE\" RUST_LOG=\"$LOG_LEVEL\" $WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/$BUILD_TYPE/nos-cli \
    --config $WORKSPACE_ROOT/config/$CONFIG.toml --workload $WORKSPACE_ROOT/config/workloads/$WORKLOAD.toml \
    --policy $POLICY --bind-core $CORE_LIST $DUMP_OPTION --sm-port $SMPORT"
