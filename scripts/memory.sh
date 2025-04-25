#!/bin/bash

TMUX_SERVER="nos-server"

run_server() {
    local EXECUTABLE=$1
    local CONFIG=$2

    tmux kill-session -t $TMUX_SERVER >> /dev/null 2>&1
    tmux new-session -s $TMUX_SERVER -n server -d
    tmux send-keys -t $TMUX_SERVER "./basic/svr.sh -c $CONFIG -e $EXECUTABLE -r" C-m

    # Compilation is slow, but shouldn't take more than 90s.
    RETRY_CNT=0
    while [[ $RETRY_CNT -le 90 ]]; do
        if [[ $(tmux capture-pane -t $TMUX_SERVER -p | grep "cluster is ready!") ]]; then
            RETRY_CNT=99999
            break
        fi
        sleep 1
        RETRY_CNT=$((RETRY_CNT+1))
    done

    if [[ $RETRY_CNT -eq 99999 ]]; then
        echo "Server is ready."
    else
        ERROR=$(tmux capture-pane -t $TMUX_SERVER -p)
        tmux send-keys -t $TMUX_SERVER C-c
        tmux send-keys -t $TMUX_SERVER C-c
        sleep 1
        tmux kill-session -t $TMUX_SERVER >> /dev/null 2>&1
        ./utils/kill.sh

        echo " - Failed to start server: $EXECUTABLE $CONFIG"
        echo " - Last pane capture:"
        echo "$ERROR"
        return 1
    fi
}

# for system in "nos" "bsl-cocytus" "bsl-pq" "bsl-split" "bsl-repl" "bsl-repl-handoff" "bsl-dynbackup" "bsl-split-dyn"; do
for system in "bsl-dynbackup"; do
    for config in "c42" "c63"; do
        if [[ $system == "bsl-pq" && $config == "c63" ]]; then
            continue
        fi
        if [[ $system == "bsl-repl" && $config == "c62" ]]; then
            continue
        fi

        for value in "64" "256" "1k" "4k"; do
            echo "Running $system $config $value ..."

            run_server $system $config
            if [[ $? -ne 0 ]]; then
                echo "Exiting."
                exit 1
            fi

            ./dist-cli/run-prepare.sh $value
            MEMORY_KB=$(utils/get-memory.sh $system)
            echo "$system $config $value $MEMORY_KB" >> memory.txt

            # Kill the server.
            tmux send-keys -t $TMUX_SERVER C-c
            tmux send-keys -t $TMUX_SERVER C-c
            sleep 1
            tmux kill-session -t $TMUX_SERVER >> /dev/null 2>&1
            ./utils/kill.sh
            sleep 1
        done

        ./utils/kill.sh
        sleep 5
    done
done
