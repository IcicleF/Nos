# How to use this repository

This repository consists of the following parts:

```
Nos
├── config
├── data
├── ff-sbibd
├── isal-sys
├── motivation
├── nos
├── nos-cli
├── numa-sys
├── rrppcc
├── scripts
├── target
└── trace-preprocess
```

## Source Code

> If you, as an experienced system developer, want to directly dive into the code, we recommend that you first run `cargo doc` and read the generated documentation of `nos` and `nos_cli`.

Nostor adopts a C/S architecture.
The server and the client belong to different Rust projects.

### Server

In the `nos` directory, you can find the source code of the server-side components.


### Client

### Others

* `rrppcc` is the RPC engine that Nostor uses.
* `isal-sys` and `numa-sys` are Rust bindings to ISA-L and libnuma, respectively.
