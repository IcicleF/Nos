#!/usr/bin/python3

import sys


def compute_rate(offered_load: int, num_clients: int, num_threads: int) -> int:
    """
    Compute the open-loop request rate (lambda, unit: nanoseconds) for a given offered load.
    """

    # Minus 1 here because we need one closed-loop thread to measure the latency.
    # This thread can only offer very limited throughput.
    load_per_thread = offered_load / (num_clients * num_threads - 1)
    rate = round(1e9 / load_per_thread)
    return int(rate)


if __name__ == '__main__':
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <offered_load> <num_clients> <num_threads>")
        sys.exit(1)

    # parse possible K/M/G suffix
    offered_load_str = sys.argv[1]
    suffix = offered_load_str[-1]
    offered_load = int(offered_load_str[:-1]) * 10**3 if suffix in 'kK' else \
        int(offered_load_str[:-1]) * 10**6 if suffix in 'mM' else \
        int(offered_load_str[:-1]) * 10**9 if suffix in 'gG' else \
        int(offered_load_str)

    num_clients = int(sys.argv[2])
    num_threads = int(sys.argv[3])

    rate = compute_rate(offered_load, num_clients, num_threads)
    print(rate)
    