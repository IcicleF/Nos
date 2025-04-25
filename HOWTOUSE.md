# How to use this repository

This repository consists of the following parts:

```shell
Nos
├── config           # Cluster & experiment workload configs
├── data             # Original & processed experiment data
├── ff-sbibd         # Theory verification of SBIBD generation
├── isal-sys         # Bindings for Intel ISA-L
├── motivation       # Verification of XOR and EC encode bandwidth
├── nos              # Nos server
├── nos-cli          # Nos client
├── numa-sys         # Bindings for libnuma
├── rrppcc           # RPC engine
├── scripts          # Experiment scripts
├── trace-preprocess # Twemcache trace preprocessor
└── ...
```

## Source Code

> If you, as an experienced system developer, want to directly dive into the code, we recommend that you first run `cargo doc` and read the generated documentation of `nos` and `nos_cli`.

Nostor adopts a C/S architecture.
The server and the client belong to different Rust projects.

### Server

In the `nos` directory, you can find the source code of the server-side components.

```shell
nos/src
├── baselines # Baselines
├── bin       # Utilities (executable)
├── ctrl      # Control plane
├── ec        # Erasure coding module
├── objstore  # **Nostor**
├── utils     # Utilities
└── ...
```

Nostor uses [DashMap](https://docs.rs/dashmap) as the in-memory key-value store implementation.

### Client

In the `nos-cli` directory, you can find the client.
The client fits Nos and all baselines.

```shell
nos-cli/src
├── bin            # Utilities (executable)
├── distributed.rs # RPC wrappers
├── perf           # Performance monitor
├── runners        # Workload runner
├── utils          # Utilities
├── workload       # Workload generator
└── ...
```


## Experiments

### AE Scripts

AE scripts reside in the `scripts/ae-*` repositories.
For more details, see the repository [README](README.md).

#### Killing a Running Experiment

```shell
scripts/utils/kill.sh     # Kill everything, in the entire cluster.
scripts/utils/kill.sh cli # Kill only the clients.
```


### Configuration Files

Configurations are in the `config` directory.
Servers and clients will read them and determine how to communicate with each other / run the workload.

```shell
config
├── xxx.toml     # Cluster configuration
└── workloads
    └── yyy.toml # Workload configuration
```


### Original Data

The original data used by the paper is in `data`.


## Proof-of-Concept (POC)

* `ff-sbibd` contains code that generates SBIBDs based on finite field theory.
  Nos uses brute force search to find SBIBDs, but that's only effective when $k$ is small.
  For larger $k$ values, you may use the code in `ff-sbibd` instead.
  Experiments on WSL proves that it can generate SBIBDs for $k \leq 20$ in a few seconds.
* `motivation` contains simple C++ code that verifies ISA-L XORs data faster than EC encode.
  This was a motivation of Nos in a previous version of paper.
  Anyway, it is a benefit os Nos, isn't it :)

