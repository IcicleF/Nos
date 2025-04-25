#!/bin/bash

# Estimated run time: ~70 minutes

SCRIPT_DIR=$(dirname $(readlink -f "$0"))

for exp in "3" "4" "5" "6" "8" "9" "11"; do
    echo "Running script fig$exp..."
    bash ./fig$exp.sh
    
    $SCRIPT_DIR/../utils/kill.sh >> /dev/null 2>&1
    sleep 5
done

echo "Done."
