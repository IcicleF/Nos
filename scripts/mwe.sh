#!/bin/bash

SCRIPT_DIR=$(dirname $(readlink -f "$0"))
TMUX_SERVER="nos-mwe-server"

# Setup tmux sessions for the server and the client.
tmux kill-session -t $TMUX_SERVER >> /dev/null 2>&1
tmux new-session -s $TMUX_SERVER -n server -d

# Start the server.
echo "- Starting the server, may take a while due to compilation ..."
tmux send-keys -t $TMUX_SERVER "$SCRIPT_DIR/basic/svr.sh -c cloudlab-small-perf" C-m

while true; do
    if [[ $(tmux capture-pane -t $TMUX_SERVER -p | grep "cluster is ready!") ]]; then
        break
    fi
    sleep 1
done
echo "- Server is ready."

# Start the client.
echo "- Starting the client, may take a while due to compilation ..."
echo
$SCRIPT_DIR/basic/cli.sh -c cloudlab-small-perf -w readonly-mwe -s

# Kill the server.
echo
echo "- Client finished, killing the servers ..."
tmux send-keys -t $TMUX_SERVER C-c
tmux send-keys -t $TMUX_SERVER C-c

sleep 1
tmux kill-session -t $TMUX_SERVER >> /dev/null 2>&1
echo "- Servers killed."

# Cleanup.
echo "- Performing cleanup ..."
pdsh -w ssh:node[0-6] "killall -9 nos >> /dev/null 2>&1" >> /dev/null 2>&1
pdsh -w ssh:node[0-6] "rm -rf /tmp/nos_* >> /dev/null 2>&1" >> /dev/null 2>&1
echo "- Cleanup finished."
echo
echo ":) MWE done!"
