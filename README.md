# Reno

[![Crates.io](https://img.shields.io/crates/v/reno?style=for-the-badge&logo=rust)](https://crates.io/crates/reno)
[![Crates.io](https://img.shields.io/crates/d/reno?style=for-the-badge&logo=rust)](https://crates.io/crates/reno)

Reno is an experimental Linux container runtime that implements the [OCI runtime specification](https://github.com/opencontainers/runtime-spec) with Rust. It supports a subset of the specification's features, including namespaces, capabilities, mounts, and hooks.

## Installation

```console
cargo install reno
```

## Usage

```console
sudo reno create example_container --bundle /path/to/bundle
sudo reno start example_container
sudo reno state example_container
sudo reno delete example_container
```
