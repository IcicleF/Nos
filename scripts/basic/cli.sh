#!/bin/bash

BUILD_TYPE="debug"
LOG_LEVEL="info"
CONFIG="${NOS_CONFIG:-lab}"
WORKLOAD="readonly-mwe"
POLICY="closed"
DUMP=0
CORE_LIST="18-23,42-47"
SMPORT="31850"

PREPARE_OPTION="--prepare 0/1"

VALID_ARGS=$(getopt -o hrdsc:w:p:b:l: --long help,release,dump,silent,prepare:,config:,workload:,policy:,bind:,log:,port: -- "$@")
if [[ $? -ne 0 ]]; then
    exit 1;
fi

eval set -- "$VALID_ARGS"
while [ : ]; do
    case $1 in
        -h | --help)
            echo "Nos client bootstrap script"
            echo
            echo "Usage: cli.sh [OPTIONS]"
            echo
            echo "Options:"
            echo "  -h, --help                 Print this help message and exit."
            echo "  -r, --release              Run release version instead of debug."
            echo "  -d, --dump                 Dump performance stats to file."
            echo "  -s, --silent               Dump to /dev/null and suppress logs. This option conflicts with \`--dump\`."
            echo "      --prepare <I>/<N>      Do dataset preparation (i of n). If there is no slash in the argument, do not prepare."
            echo "  -c, --config <CONFIG>      Specify config file name (only name, without \`.toml\` suffix). [default: lab]"
            echo "                             Config files will be looked up in ./config/ directory."
            echo "                             If \`NOS_CONFIG\` is set, it will be used as the config file name."
            echo "  -w, --workload <WORKLOAD>  Specify workload file name (only name, without \`.toml\` suffix). [default: small]"
            echo "                             Workload files will be looked up in ./config/workloads/ directory."
            echo "  -p, --policy <POLICY>      Specify execution policy. [default: closed]"
            echo "                             When this option is specified, the workload details will be ignored."
            echo "  -b, --bind <CORELIST>      Specify the cores to bind on. [default: 18-23,42-47]"
            echo "  -l, --log <LEVEL>          Specify log level. This option conflicts with \`--silent\`. [default: info]"
            echo "      --port <PORT>          Specify rrppcc port. [default: 31850]"
            exit 0
            ;;
        -r | --release)
            BUILD_TYPE="release"
            shift
            ;;
        -d | --dump)
            if [[ $DUMP == 2 ]]; then
                echo "error: --dump and --silent cannot be specified at the same time."
                exit 1
            fi
            DUMP=1
            shift
            ;;
        -s | --silent)
            if [[ $DUMP == 1 ]]; then
                echo "error: --dump and --silent cannot be specified at the same time."
                exit 1
            fi
            DUMP=2
            shift
            ;;
        --prepare)
            if [[ $2 == *"/"* ]]; then
                PREPARE_OPTION="--prepare $2"
            else
                PREPARE_OPTION=""
            fi
            shift 2
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
        -l | --log)
            LOG_LEVEL=$2
            shift 2
            ;;
        --port)
            SMPORT=$2
            shift 2
            ;;
        --) shift;
            break
            ;;
    esac
done

# Print configuration.
echo "build type:  $BUILD_TYPE"
echo "config:      $CONFIG"
echo "policy:      $POLICY"

if [[ $PREPARE_OPTION != "" ]]; then
    echo "prepare:     yes"
else
    echo "prepare:     no"
fi

case $DUMP in
    0)
        echo "log level:   $LOG_LEVEL"
        echo "dump:        no"
        ;;
    1)
        echo "log level:   $LOG_LEVEL"
        echo "dump:        yes"
        ;;
    2)
        echo "log level:   off (suppressed)"
        echo "dump:        /dev/null"
        ;;
esac

echo "bind cores:  $CORE_LIST"
echo

# Build and run.
SCRIPT_DIR=$(dirname $(readlink -f "$0"))
WORKSPACE_ROOT=$(python3 $SCRIPT_DIR/../utils/find-root.py)

BUILD_OPTIONS="--package nos-cli"
if [[ $BUILD_TYPE == "release" ]]; then
    BUILD_OPTIONS="$BUILD_OPTIONS --release"
fi
if [[ $DUMP == 2 ]]; then
    BUILD_OPTIONS="$BUILD_OPTIONS --quiet"
fi
$SCRIPT_DIR/../utils/build.sh $BUILD_OPTIONS

if [[ $? -ne 0 ]]; then
    echo "compilation failed, abort."
    exit 1
else
    echo "compilation finished."
fi
echo

case $DUMP in
    0)
        RUST_LOG=$LOG_LEVEL $WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/$BUILD_TYPE/nos-cli \
            --config $WORKSPACE_ROOT/config/$CONFIG.toml --workload $WORKSPACE_ROOT/config/workloads/$WORKLOAD.toml \
            --policy $POLICY $PREPARE_OPTION \
            --bind-core $CORE_LIST --sm-port $SMPORT 
        ;;
    1)
        mkdir -p $WORKSPACE_ROOT/data

        TIMESTAMP=$(date +'%Y%m%d-%H%M%S')
        mkdir -p $WORKSPACE_ROOT/data/uncollected
        mkdir -p $WORKSPACE_ROOT/data/uncollected/$WORKLOAD-$TIMESTAMP/

        RUST_LOG=$LOG_LEVEL $WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/$BUILD_TYPE/nos-cli \
            --config $WORKSPACE_ROOT/config/$CONFIG.toml --workload $WORKSPACE_ROOT/config/workloads/$WORKLOAD.toml \
            --policy $POLICY $PREPARE_OPTION \
            --bind-core $CORE_LIST --sm-port $SMPORT --dump $WORKSPACE_ROOT/data/uncollected/$WORKLOAD-$TIMESTAMP/dump.csv
        ;;
    2)
        RUST_LOG=off $WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/$BUILD_TYPE/nos-cli \
            --config $WORKSPACE_ROOT/config/$CONFIG.toml --workload $WORKSPACE_ROOT/config/workloads/$WORKLOAD.toml \
            --policy $POLICY $PREPARE_OPTION \
            --bind-core $CORE_LIST --sm-port $SMPORT --dump /dev/null
        ;;
esac
