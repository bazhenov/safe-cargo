[![Crates.io](https://img.shields.io/crates/v/cargo-safe)
](https://crates.io/crates/cargo-safe)
[![GitHub License](https://img.shields.io/github/license/bazhenov/cargo-safe)](https://github.com/bazhenov/cargo-safe?tab=MIT-1-ov-file#readme)

# Problem

Supply chain attacks became very common thing these days, but we're still running untrusted code on our machines everyday. This crate provides `cargo safe` subcommand, that runs all commands in a sandboxed environment.

For now it is working on macOS only using Apple's sandboxing mechanism.

# How to use it?

## Installation

```console
$ cargo install cargo-safe
```

Using is pretty simple, you can use any `cargo` command:

```console
$ cargo safe buld
$ cargo safe test
$ cargo safe run
```

Or any other cargo command.

# What is allowed inside sandoxed environment

## Read access

Sandobx allow access to list all files (without reading their content), and read/execute following files and directories:

 - `/dev/random` and `/dev/urandom`
 - `/dev/tty`
 - All files in `PATH` directiories
 - All files in following directories (and subdirectories):
    - `/private/etc/`
    - `/private/var/db/timezone/`
    - `/Applications/Xcode.app/Contents/Developer`
    - `/usr/lib/`
    - `/usr/lib/info/`
    - `/private/var/db/dyld/`
    - `/System/Library/Frameworks/`
    - `/System/Library/PrivateFrameworks/`
    - `/System/Library/`
    - `/System/Volumes/Preboot/Cryptexes/OS`
    - `/System/Cryptexes/OS/`
    - `/Library/Preferences/`


## Write access

 - OS temporary directory
 - `cargo` and `target` directories private to a sandbox (separate from `$HOME/.cargo` and `target` in your workdir)
 - `Cargo.lock` in your project directory – otherwise it's impossible to build a project

## Network access

 - communication over `/private/var/run/mDNSResponder` – to allow DNS lookups
 - outbound network connections to ports 80/443 - to download crates

Full list of permissions can be found in [sources](https://github.com/bazhenov/cargo-safe/blob/e30912c7c545e1565142f145420eba87d1f1b299/src/main.rs#L45-L157).
