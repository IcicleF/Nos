#!/bin/bash

CARGO_BUILD_OPTIONS="--target x86_64-unknown-linux-gnu"

VALID_ARGS=$(getopt -o hrqp: --long help,release,quiet,package: -- "$@")
if [[ $? -ne 0 ]]; then
    exit 1;
fi

eval set -- "$VALID_ARGS"
while [ : ]; do
    case $1 in
        -h | --help)
            echo "Build script"
            echo
            echo "Usage: build.sh [OPTIONS]"
            echo
            echo "Options:"
            echo "  -h, --help                 Print this help message and exit."
            echo "  -r, --release              Build release version instead of debug."
            echo "  -p, --package <PACKAGE>    Specify package to build. If not specified, build all packages."
            exit 0
            ;;
        -r | --release)
            CARGO_BUILD_OPTIONS="$CARGO_BUILD_OPTIONS --release"
            shift
            ;;
        -q | --quiet)
            CARGO_BUILD_OPTIONS="$CARGO_BUILD_OPTIONS --quiet"
            shift
            ;;
        -p | --package)
            CARGO_BUILD_OPTIONS="$CARGO_BUILD_OPTIONS --package $2"
            shift 2
            ;;
        --) 
            shift;
            break
            ;;
    esac
done

# Build.
if [[ $CARGO_BUILD_OPTIONS != *"--package"* ]]; then
    CARGO_BUILD_OPTIONS="$CARGO_BUILD_OPTIONS --workspace"
fi
cargo +nightly build $CARGO_BUILD_OPTIONS
