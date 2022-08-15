# cli

This example implements a very basic CLI for [Axiom](https://axiom.co).

> **Warning**: This is meant to show some examples on how to call the various methods.
> Please don't actually use this, we have an 
> [official CLI](https://github.com/axiomhq/cli).

## Prerequisites

You'll need an account at [Axiom](https://cloud.axiom.co) and a personal token.

## Start the example

```sh
export AXIOM_TOKEN=<your personal token>
export AXIOM_ORG_ID=<your axiom org id>
cargo run
```

## Usage

```
$ cd examples/cli && cargo run
cli 0.1.0

USAGE:
    cli <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    datasets    Manipulate datasets
    help        Prints this message or the help of the given subcommand(s)
    users       Work with users
```

You can run something like `cargo run -- datasets --help` to get more 
information about subcommands and available flags.