# Personnummer

[![Crates.io](https://img.shields.io/crates/v/personnummer.svg)](https://crates.io/crates/personnummer)
[![Rust](https://github.com/personnummer/rust/actions/workflows/rust.yml/badge.svg)](https://github.com/personnummer/rust/actions/workflows/rust.yml)

Validate Swedish [personal identity
numbers](<https://en.wikipedia.org/wiki/Personal_identity_number_(Sweden)>) with
[Rust](https://www.rust-lang.org/).

## Usage

```rust
use personnummer::Personnummer;

fn main() {
    match Personnummer::new("199001011-0017") {
        Ok(pnr) => println!("{}: {}", pnr.format().long(), pnr.valid()),
        Err(e) => panic!("Error: {}", e),
    }
}
```

Fore more details, see [examples](examples) and/or run

```sh
cargo run --example personnummer <personnummer>
```

## In memoriam

Fredrik "Frozzare" Forsmo (1991-2026) was the initiator, co-founder and a core
contributor of the personnummer project. This library carries his work. He is
missed.
