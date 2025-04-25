# Run a normal experiment once.
# Signature: run_once <SERVER_CMD> <CLIENT_CMD> <POLICY>
# Allowed policies:
#  - nostart: Do not start the server, reuse the existing one.
#  - nokill: Do not kill the server after the experiment.
run_once() {
    local TMUX_SERVER="nos-server"
    local SCRIPTS_DIR=$(dirname $(readlink -f "$0"))

    # Go through the parent chain until the root directory (the one containing `Cargo.lock`).
    # Resolve the `scripts` directory.
    while [[ ! -f "$SCRIPTS_DIR/Cargo.lock" ]]; do
        SCRIPTS_DIR=$(dirname "$SCRIPTS_DIR")
        if [[ "$SCRIPTS_DIR" == "/" ]]; then
            echo "Error: Unable to find Cargo.lock in parent directories of cwd."
            echo "Please run this script within the Nos repository."
            exit 1
        fi
    done
    SCRIPTS_DIR="$SCRIPTS_DIR/scripts"

    local SERVER_CMD="$1"
    local CLIENT_CMD="$2"
    local POLICY="$3"
    local RETRY_CNT
    
    if [[ -z "$SERVER_CMD" || -z "$CLIENT_CMD" ]]; then
        echo "BUG FOUND: run_once called with invalid arguments."
        echo "Expected Usage: run_once <SERVER_CMD> <CLIENT_CMD>"
        echo
        echo "If you are an AE reviewer, please ask the authors to fix this."
        exit 1
    fi

    # Launch the server.
    if [[ $SESSION_POLICY != *"reuse"* ]]; then
        while true; do
            tmux kill-session -t $TMUX_SERVER >> /dev/null 2>&1
            tmux new-session -s $TMUX_SERVER -n server -d
            tmux send-keys -t $TMUX_SERVER "$SERVER_CMD" C-m

            # Wait until server is ready.
            RETRY_CNT=0
            while [[ $RETRY_CNT -le 120 ]]; do
                if [[ $(tmux capture-pane -t $TMUX_SERVER -p | grep "cluster is ready!") ]]; then
                    RETRY_CNT=99999
                    break
                fi
                if [[ $(tmux capture-pane -t $TMUX_SERVER -p | grep "ssh exited") ]]; then
                    break
                fi

                sleep 1
                RETRY_CNT=$((RETRY_CNT+1))
            done

            # Break if successfully launched.
            if [[ $RETRY_CNT -eq 99999 ]]; then
                break
            else
                tmux send-keys -t $TMUX_SERVER C-c
                tmux send-keys -t $TMUX_SERVER C-c
                sleep 1
                $SCRIPTS_DIR/utils/kill.sh
                echo "Server failed to start. Retrying..."
            fi
        done
    fi

    # Run the client.
    set -e
    eval "$CLIENT_CMD"
    set +e

    if [[ $SESSION_POLICY != *"nokill"* ]]; then
        # Stop the server.
        tmux send-keys -t $TMUX_SERVER C-c
        tmux send-keys -t $TMUX_SERVER C-c
        sleep 1
        tmux kill-session -t $TMUX_SERVER >> /dev/null 2>&1

        # Cleanup.
        $SCRIPTS_DIR/utils/kill.sh >> /dev/null 2>&1
    else
        $SCRIPTS_DIR/utils/kill.sh cli >> /dev/null 2>&1
    fi

    # Grace period.
    sleep 1
    return 0
}
