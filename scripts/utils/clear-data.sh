#!/bin/bash
# Clear uncollected data.

set -e

SCRIPT_DIR=$(dirname $(readlink -f "$0"))
CLEAR_ALL=0

while [[ $# -gt 0 ]]; do
    case $1 in
        -h | --help)
            echo "Clear the uncollected data directory (only empty subdirs by default)"
            echo
            echo "Usage: clear-data.sh [options]"
            echo
            echo "Options:"
            echo "  -h, --help   Show this help message and exit"
            echo "  -a, --all    Clear all uncollected data, not only empty subdirs"
            shift
            ;;
        -a | --all)
            CLEAR_ALL=1
            shift
            ;;
        --)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

DATA_DIR=$SCRIPT_DIR/../../data/uncollected

if [[ $CLEAR_ALL -eq 1 ]]; then
    rm -rf $DATA_DIR/*
else
    find $DATA_DIR -mindepth 1 -maxdepth 1 -type d -empty -delete
fi
echo "Done."