# Contributing

## Prerequisites

- [Rust toolchain](https://rustup.rs/)
- [just](https://github.com/casey/just)
- [bun](https://bun.com/)

Both `just` and `bun` should be managed with [asdf](https://asdf-vm.com/) -- see [.tool-versions](./.tool-versions).

## Setup

Run the following to install global Rust tools and `djlint`.

```
just setup-rust
just setup-djlint
```

## Install dependencies

```
bun install
cargo build
```

## Lint

```
just lint-rust
just lint-rust-fix

just lint-js
just lint-js-fix

just lint-html
```
