#!/bin/bash

BUILD_TYPE="debug"
LOG_LEVEL="info"
CONFIG="${NOS_CONFIG:-lab}"
EXECUTABLE="nos"

VALID_ARGS=$(getopt -o hrc:e:l: --long help,release,config:,executable:,log: -- "$@")
if [[ $? -ne 0 ]]; then
    exit 1;
fi

eval set -- "$VALID_ARGS"
while [ : ]; do
    case $1 in
        -h | --help)
            echo "Nos server bootstrap script"
            echo
            echo "Usage: svr.sh [OPTIONS]"
            echo
            echo "Options:"
            echo "  -h, --help               Print this help message and exit."
            echo "  -r, --release            Run release version instead of debug."
            echo "  -c, --config <CONFIG>    Specify config file name (only name, without \`.toml\` suffix). [default: default]"
            echo "                           Config files will be looked up in ./config/ directory."
            echo "                           If \`NOS_CONFIG\` is set, it will be used as the config file name."
            echo "  -e, --executable <NAME>  Specify executable name. [default: nos]"
            echo "  -l, --log <LEVEL>        Specify log level. [default: info]"
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
        -e | --executable)
            EXECUTABLE=$2
            shift 2
            ;;
        -l | --log)
            LOG_LEVEL=$2
            shift 2
            ;;
        --) shift;
            break
            ;;
    esac
done

echo "build type:      $BUILD_TYPE"
echo "config:          $CONFIG"
echo "executable name: $EXECUTABLE"
echo "log level:       $LOG_LEVEL"
echo

# Build and run.
SCRIPT_DIR=$(dirname $(readlink -f "$0"))
WORKSPACE_ROOT=$(python3 $SCRIPT_DIR/../utils/find-root.py)

BUILD_OPTIONS="--package nos"
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

UUID=$(uuidgen)
SSH_TARGETS=$($WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/$BUILD_TYPE/cluster2ip $WORKSPACE_ROOT/config/$CONFIG.toml)

echo "starting processes..."
echo "note: if you do not see the ready message, please abort and retry."
echo

pdsh -w ssh:$SSH_TARGETS \
    "RUST_BACKTRACE=1 RUST_LOG=$LOG_LEVEL $WORKSPACE_ROOT/target/x86_64-unknown-linux-gnu/$BUILD_TYPE/$EXECUTABLE \
        -c $WORKSPACE_ROOT/config/$CONFIG.toml -u $UUID"
    
if [[ $? -eq 0 ]]; then
    echo "servers gracefully exited."
fi

# Cleanup.
echo
echo "killing processes..."
pdsh -w ssh:$SSH_TARGETS "killall -9 $EXECUTABLE >> /dev/null 2>&1" >> /dev/null 2>&1
if [[ $? -ne 0 ]]; then
    echo "kill failed, you need to manually kill the processes!"
    exit 1;
fi

pdsh -w ssh:$SSH_TARGETS "rm -rf /tmp/nos_* >> /dev/null 2>&1" >> /dev/null 2>&1
if [[ $? -ne 0 ]]; then
    echo "clean failed, you need to manually clean the temporary files!"
    exit 1;
fi
