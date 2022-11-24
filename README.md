# Reno

[![Crates.io](https://img.shields.io/crates/v/reno?style=for-the-badge&logo=rust)](https://crates.io/crates/reno)
[![Crates.io](https://img.shields.io/crates/d/reno?style=for-the-badge&logo=rust)](https://crates.io/crates/reno)

Reno is an experimental Linux container runtime that implements the [OCI runtime specification](https://github.com/opencontainers/runtime-spec) with Rust. Reno supports a subset of features described in the specification, such as namespaces, capabilities, mounts, and hooks.

## Installation

```shell
cargo install reno
```

## Usage

```console
sudo reno create example_container --bundle /path/to/bundle
sudo reno start example_container
sudo reno state example_container
sudo reno delete example_container
```
