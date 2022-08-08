# ingest-hn

This example ingests all HN posts up into a dataset.

## Prerequisites

You'll need an account at [Axiom](https://cloud.axiom.co), a dataset and an API
token that can ingest into that dataset.

## Start the example

```sh
export DATASET_NAME=<your dataset name>
export AXIOM_TOKEN=<your api token>
cargo run
```