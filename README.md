# Stripeless Erasure Coding

Welcome to the artifact repository of the OSDI'25 accepted paper: _Stripeless Data Placement for Erasure-Coded In-Memory Storage_.

> **To AE reviewers:** Should there be any questions, please contact the authors in HotCRP.
> The authors will respond to each question as soon as possible and not later than 24 hours.
>
> The authors expect that they will prepare an AE testbed on CloudLab during kick-the-tires response period (May 5, 2025 - May 12, 2025). 
> Details will be on HotCRP.
> Before or after that period, we kindly ask the reviewer to create testbeds manually.

## Table of Content

- [Artifact Overview](#artifact-overview)
- [Environment Setup](#environment-setup)
  - [For CloudLab users](#for-cloudlab-users)
  - [For others](#for-others)
- [Minimum Working Example (MWE)](#minimum-working-example-mwe)
    - [Troubleshooting](#troubleshooting)
        - [Compilation error](#compilation-error)
        - [Tweaking NIC name or port ID](#tweaking-nic-name-or-port-id)
        - [Modifying cluster configuration](#modifying-cluster-configuration)
- [Evaluating the Artifact](#evaluating-the-artifact)
    - [Reproducing the results](#reproducing-the-results)
- [Reusing the Code](#reusing-the-code)


## Artifact Overview

This artifact is a repository that contains the source code of Nos and baselines, scripts and configuration files to reproduce experiments in the paper, the original data, and other components (proof-of-concepts, trace preprocessors, etc.).

* **Source code:** mainly in `nos/` (server-side) and `nos-cli/` (client-side).
  `rrppcc/` is the RPC engine; its details are irrelavent here.
  `isal-sys/` and `numa-sys/` are Rust bindings to ISA-L and libnuma, respectively.
* **Scripts:** in `scripts/`.
  They make use of configuration files in `config/` to bootstrap the cluster.
* **Original data:** in `data/`.
* **Others:** in `ff-sbibd/`, `motivation/`, and `trace-preprocess/`.


## Environment Setup

### For CloudLab users

* Create an experiment profile with [scripts/cloudlab/profile.py](scripts/cloudlab/profile.py).
    * You may also directly use the profile link: https://www.cloudlab.us/p/dandelion/c6525-flex
* Reserve nodes on CloudLab and instantiate an experiment.
    * To reproduce the experiments, **16 c6525-100g nodes are necessary.**
    * To verify the functionalties, 7 c6525-25g nodes are enough.
        * It is possible to use even fewer nodes or other node types (**RDMA NICs are necessary!**), but in that case you must [tweak the configurations](#tweaking-nic-name-or-port-id).
* On `node0` of your experiment, create a `prepare.sh` in your home directory. Copy the contents of [scripts/cloudlab/prepare.sh](scripts/cloudlab/prepare.sh) in it.
* Run `prepare.sh`.
    * If the cluster size is not 16, run `prepare.sh X` instead, where `X` is the cluster size. For example, run `prepare.sh 7` in a 7-node cluster.
    * There is an immediate confirmation after running `prepare.sh`. Do not leave immediately after running the script.
      After the confirmation, the script will run for about 10 minutes.
* **Everything is OK!** Go to the `project-nos` directory.
    * `prepare.sh` will also setup zsh. It is recommended that you use zsh instead of the default bash.
    * If you use VSCode, the repository also provides a recommended plugin list for you.

### For others

* Install dependencies by yourself:
    * Rust (latest version, **nightly required**)
    * [ISA-L](https://github.com/intel/isa-l.git)
    * [DOCA-OFED](https://developer.nvidia.com/doca-downloads?deployment_platform=Host-Server&deployment_package=DOCA-Host&target_os=Linux&Architecture=x86_64&Profile=doca-ofed)
    * `libclang-dev`, `libnuma-dev`, `uuid-runtime`
* Rename your cluster nodes to `node0`, `node1`, ..., `node{N-1}`. All nodes must trust `node0`'s SSH authenticity.
    * If you don't want to do so, you may need to [tweak the cluster and workload configurations](#modifying-cluster-configuration).
* Clone the repository and make sure that it is shared by NFS over the entire cluster. **Other nodes must have write permission to the repository directory over NFS!**
* Go to the cloned repository and test if `cargo build` succeeds.
* Run `scripts/distribute-traces
* Modify `MAXNODE` in [scripts/dist-cli/run-prepare.sh](scripts/dist-cli/run-prepare.sh) and [scripts/utils/kill.sh](scripts/utils/kill.sh) to your cluster size **minus one**.
* **Everything should be OK now!** 


## Minimum Working Example (MWE)

We strongly recommend that you run the MWE before everything else.
It works only if your cluster **has at least 7 nodes**.

```shell
scripts/mwe.sh
```

If the scripts finishes after printing `:) MWE done!`, then everything is properly set up, and you can start evaluating the artifact.
Otherwise, please refer to the following text for troubleshooting.

> **To AE reviewers:** A success in running the MWE does NOT guarantee that the artifact is fully functional,
> as the MWE only runs Nos (without baselines) under a microbenchmark with no failures.

### Troubleshooting

#### Compilation error

Usually this should not happen if you have the latest versions of DOCA-OFED, Intel ISA-L, Rust, and all necessary dependency libraries installed.
Check if you have installed the dependencies (see the "[For others](#for-others)" section above).

If you keep getting a compilation error, file an issue to the authors.

#### Tweaking NIC name or port ID

Nos defaults to using `mlx5_2`, physical port 1.
This device and port corresponds to the 100Gbps port on a c6525-100g node, or the second 25Gbps port on a c6525-25g node.

If you are using a customized cluster, you may want to tweak these configurations.
In that case, please run [scripts/utils/tweak-nic.sh](scripts/utils/tweak-nic.sh).

#### Modifying cluster configuration

Nos relies on cluster configuration files to bootstrap the storage cluster.
These files are of the TOML format, and they reside in the `config` directory.
**If you are using a 16-node c6525-100g cluster, you can leave them intact.**
Otherwise, you may want to modify the configuration files to match your cluster setup.

To understand how Nos parses the configuration file, please go to Lines 56-62 of [nos/src/lib.rs](nos/src/lib.rs).
Specifically, please refer to the doc comment of `Cluster::load_toml` at Lines 317-443 of [nos/src/ctrl/cluster.rs](nos/src/ctrl/cluster.rs) to understand the syntax of the cluster configuration.

#### Modifying workloads

Workload configuration files reside in `config/workloads`.
They are also in the TOML format.
Please refer to [nos-cli/src/workload/defs.rs](nos-cli/src/workload/defs.rs) for the syntax of the workload configuration files.


## Evaluating the Artifact

> Request badges for OSDI'25 AE: **Artifact Functional**

Go to `scripts/ae-functional` and run `run-all.sh`.
It will run a subset of the experiments presented in the paper (covering all figures) and will take ~70 minutes to finish.

### Reproducing the results

> The reproduced badge is not request by the authors.

Go to `scripts/ae-functional` and run `run-all.sh`.
It should take several hours to finish; the exact run time is unknown by now.

The figure plotting scripts are not yet available.


## Reusing the Code

See our [How-to-Use Documentation](HOWTOUSE.md) for a guide through this repository.
You can also directly read the code -- they are well documented.

If you are only interested in our RDMA/RPC engine, please refer to their documentation: [rrddmma](https://docs.rs/rrddmma), [rrppcc](https://docs.rs/rrppcc).
They are still immature and under development, and there might be significant API changes in the future!
