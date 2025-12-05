[![Crates.io](https://img.shields.io/crates/v/safe-cargo)
](https://crates.io/crates/safe-cargo)
[![GitHub License](https://img.shields.io/github/license/bazhenov/safe-cargo)](https://github.com/bazhenov/safe-cargo?tab=MIT-1-ov-file#readme)

# Problem

Supply chain attacks became very common thing these days, but we're still running untrusted code on our machines everyday. This crate provides `safe-cargo` subcommand, that runs all commands in a sandboxed environment.

For now it is working on macOS only using Apple's sandboxing mechanism.

# How to use it?

## Installation

```console
$ cargo install safe-cargo
```

Using is pretty simple, you can use any `cargo` command:

```console
$ safe-cargo build
$ safe-cargo test
$ safe-cargo run
```

Or any other cargo command.

# What is allowed inside sandoxed environment

## Read access

Sandbox allow access to list all files (without reading their content), and read/execute following files and directories:

 - `/dev/random` and `/dev/urandom`
 - `/dev/tty`
 - All files in `PATH` directories
 - All files in following directories (and subdirectories):
    - `/private/etc/`
    - `/private/var/db/timezone/`
    - `/Applications/Xcode.app/Contents/Developer`
    - `/usr/lib/`
    - `/private/var/db/dyld/`
    - `/System/Library/`
    - `/System/Volumes/Preboot/Cryptexes/OS`
    - `/System/Cryptexes/OS/`
    - `/Library/Preferences/`


## Write access

 - OS temporary directory
 - current controlling tty
 - `cargo` and `target` directories private to a sandbox (separate from `$HOME/.cargo` and `target` in your workdir)
 - `Cargo.lock` in your project directory – otherwise it's impossible to build a project

## Network access

 - communication over `/private/var/run/mDNSResponder` – to allow DNS lookups
 - outbound network connections to ports 80/443 - to download crates

Full list of permissions can be found in [sources](https://github.com/bazhenov/safe-cargo/blob/c8b377e902d09c2e2d570b4b8ecbc3809baad739/src/lib.rs#L4).
