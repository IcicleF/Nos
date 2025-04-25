#!/bin/bash

SCRIPT_DIR=$(dirname $(readlink -f $0))

MAXNODES=$(cat $SCRIPT_DIR/../MAXNODES)
MAXNODE=$(($MAXNODES - 1))

if [[ $1 == "cli" ]]; then
    pdsh -w ssh:node[0-$MAXNODE] "killall -9 nos-cli >> /dev/null 2>&1" >> /dev/null 2>&1
    pdsh -w ssh:node[0-$MAXNODE] "$SCRIPT_DIR/free-port.sh cli >> /dev/null 2>&1" >> /dev/null 2>&1
else
    tmux kill-session -t nos-server >> /dev/null 2>&1
    pdsh -w ssh:node[0-$MAXNODE] "killall -9 nos-cli nos bsl-raid bsl-split bsl-split-lb bsl-cocytus bsl-pq bsl-repl-handoff bsl-dynbackup >> /dev/null 2>&1" >> /dev/null 2>&1
    pdsh -w ssh:node[0-$MAXNODE] "rm -rf /tmp/nos_* >> /dev/null 2>&1" >> /dev/null 2>&1
    pdsh -w ssh:node[0-$MAXNODE] "$SCRIPT_DIR/free-port.sh >> /dev/null 2>&1" >> /dev/null 2>&1
fi

echo "Killed."
