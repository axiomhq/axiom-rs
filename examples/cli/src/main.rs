use axiom_rs::{
    datasets::{ContentEncoding, ContentType},
    Client,
};
use std::time::Duration;
use structopt::StructOpt;
use tokio::io::{stdin, AsyncReadExt};

#[derive(Debug, StructOpt)]
enum Opt {
    /// Work with users.
    Users(Users),
    /// Manipulate datasets.
    Datasets(Datasets),
}

#[derive(Debug, StructOpt)]
enum Users {
    /// Get the current user
    Current,
}

#[derive(Debug, StructOpt)]
enum Datasets {
    /// List datasets
    List,
    /// Get a dataset
    Get { name: String },
    /// Get information for a dataset
    Info { name: String },
    /// Update the description of a dataset
    Update {
        name: String,

        #[structopt(long, short)]
        description: String,
    },
    /// Delete a dataset
    Delete { name: String },
    /// Trim a dataset
    Trim {
        name: String,

        #[structopt(long)]
        seconds: u64,
    },
    /// Ingest into a dataset from stdin.
    Ingest {
        name: String,

        #[structopt(long, default_value = "application/json")]
        content_type: ContentType,
        #[structopt(long, default_value = "")]
        content_encoding: ContentEncoding,
    },
    /// Query a dataset using APL.
    Query { apl: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let client = Client::new()?;

    match opt {
        Opt::Datasets(datasets) => match datasets {
            Datasets::List => client
                .datasets
                .list()
                .await?
                .into_iter()
                .for_each(|dataset| {
                    println!("{:?}", dataset);
                }),
            Datasets::Get { name } => println!("{:?}", client.datasets.get(&name).await?),
            Datasets::Info { name } => println!("{:?}", client.datasets.info(&name).await?),
            Datasets::Update { name, description } => {
                let dataset = client
                    .datasets
                    .update(
                        &name,
                        axiom_rs::datasets::DatasetUpdateRequest { description },
                    )
                    .await?;
                println!("{:?}", dataset);
            }
            Datasets::Delete { name } => client.datasets.delete(&name).await?,
            Datasets::Trim { name, seconds } => println!(
                "{:?}",
                client
                    .datasets
                    .trim(&name, Duration::from_secs(seconds))
                    .await?
            ),
            Datasets::Ingest {
                name,
                content_type,
                content_encoding,
            } => {
                let mut buf = Vec::new();
                stdin().read_to_end(&mut buf).await?;
                let ingest_status = client
                    .ingest_bytes(&name, buf, content_type, content_encoding)
                    .await?;
                println!("{:?}", ingest_status);
            }
            Datasets::Query { apl } => {
                let result = client.query(apl, None).await?;
                println!("{:?}", result);
            }
        },
        Opt::Users(users) => match users {
            Users::Current => {
                let user = client.users.current().await?;
                println!("{:?}", user);
            }
        },
    };

    Ok(())
}
