#!/bin/bash

if [ $# -ne 1 ]; then
    echo "Measure memory usage of a system"
    echo "Usage: $0 <executable>"
    exit 1
fi

EXECUTABLE=$1
PID=$(ps -ef | grep "release/$EXECUTABLE" | tail -n 3 | head -n 1 | awk '{print $2}')
RSS=$(pmap -x $PID | tail -n 1 | awk '{print $4}')
echo $RSS
