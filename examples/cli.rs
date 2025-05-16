use axiom_rs::{
    datasets::{ContentEncoding, ContentType},
    Client,
};
use cli_table::{Cell as _, Style as _, Table as _};
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
    // /// Get information for a dataset
    // Info { name: String },
    /// Update the description of a dataset
    Update {
        name: String,

        #[structopt(long, short)]
        description: String,
    },
    /// Delete a dataset
    Delete { name: String },
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
                .datasets()
                .list()
                .await?
                .into_iter()
                .for_each(|dataset| {
                    println!("{:?}", dataset);
                }),
            Datasets::Get { name } => println!("{:?}", client.datasets().get(&name).await?),
            Datasets::Update { name, description } => {
                let dataset = client.datasets().update(&name, description).await?;
                println!("{:?}", dataset);
            }
            Datasets::Delete { name } => client.datasets().delete(&name).await?,
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
                let result = client.query(&apl, None).await?;
                for table in result.tables {
                    println!("{}:", table.name);

                    let rows_iter = table.iter();
                    let mut rows = Vec::with_capacity(rows_iter.size_hint().0);
                    for row in rows_iter {
                        let field_iter = row.iter();
                        let mut row_vec = Vec::with_capacity(field_iter.size_hint().0);
                        for field in field_iter {
                            row_vec.push(field.map_or_else(
                                || "-".to_string(),
                                |v| serde_json::to_string(v).unwrap(),
                            ));
                        }
                        rows.push(row_vec);
                    }

                    let mut fields = Vec::with_capacity(table.fields.len());
                    for field in table.fields {
                        fields.push(field.name.to_string().cell().bold(true));
                    }

                    let t = rows.table().title(fields).bold(true);

                    let table_display = t.display().unwrap();
                    println!("{}", table_display);
                }
            }
        },
        Opt::Users(users) => match users {
            Users::Current => {
                let user = client.users().current().await?;
                println!("{:?}", user);
            }
        },
    };

    Ok(())
}
