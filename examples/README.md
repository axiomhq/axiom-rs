# Examples

There's two examples: cli and ingest-hn.

## Prerequisites

You'll need an account at [Axiom](https://cloud.axiom.co), a dataset and an API
token that can ingest into that dataset.

## cli

This example implements a very basic CLI for [Axiom](https://axiom.co).

> **Warning**: This is meant to show some examples on how to call the various
> methods.
> Please don't actually use this, we have an
> [official CLI](https://github.com/axiomhq/cli).

### Start the example

```sh
export AXIOM_TOKEN=<your personal token>
export AXIOM_ORG_ID=<your axiom org id>
cargo run
```

### Usage

```bash
# Run this from the project root
$ cargo run --example cli
axiom-rs 0.11.2

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

## ingest-hn

This example ingests all HN posts into an Axiom dataset.

### Start the example

```sh
export DATASET_NAME=<your dataset name>
export AXIOM_TOKEN=<your api token>
cargo run --example ingest-hn
```
