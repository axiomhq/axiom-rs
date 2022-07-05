use async_trait::async_trait;
use axiom_rs::{virtual_fields::*, Client};
use std::env;
use test_context::{test_context, AsyncTestContext};

struct Context {
    client: Client,
    dataset_id: String,
    virtual_field: VirtualField,
}

#[async_trait]
impl AsyncTestContext for Context {
    async fn setup() -> Context {
        let client = Client::new().unwrap();

        let dataset_name = format!(
            "test-axiom-rs-virtual-fields-{}",
            env::var("AXIOM_DATASET_SUFFIX").expect("AXIOM_DATASET_SUFFIX is not set"),
        );

        // Delete dataset in case we have a zombie
        client.datasets.delete(&dataset_name).await.ok();

        let dataset = client
            .datasets
            .create(
                dataset_name,
                "This is a test dataset for virtual fields integration tests.",
            )
            .await
            .unwrap();

        let virtual_field = client
            .virtual_fields
            .create(VirtualFieldCreateUpdateRequest {
                dataset: dataset.name.clone(),
                name: "status_failed".to_string(),
                description: "Failed requests".to_string(),
                expression: "response > 399".to_string(),
            })
            .await
            .unwrap();

        Context {
            client,
            dataset_id: dataset.name,
            virtual_field,
        }
    }

    async fn teardown(self) {
        self.client
            .virtual_fields
            .delete(&self.virtual_field.id)
            .await
            .unwrap();
        self.client.datasets.delete(&self.dataset_id).await.unwrap();
    }
}

#[test_context(Context)]
#[tokio::test]
async fn test_virtual_fields(&mut ctx: Context) {
    // Let's update the virtual field.
    let virtual_field = ctx
        .client
        .virtual_fields
        .update(
            ctx.virtual_field.id.clone(),
            VirtualFieldCreateUpdateRequest {
                dataset: ctx.dataset_id.clone(),
                name: "status_bad".to_string(),
                description: "Bad Requests".to_string(),
                expression: "response == 400".to_string(),
            },
        )
        .await
        .unwrap();
    ctx.virtual_field = virtual_field;

    // Get the virtual field and make sure it matches what we have updated it to.
    let virtual_field = ctx
        .client
        .virtual_fields
        .get(ctx.virtual_field.id.clone())
        .await
        .unwrap();
    assert_eq!(ctx.virtual_field, virtual_field);

    // List all virtual fields and make sure the created virtual field is part
    // of that list.
    let virtual_fields = ctx
        .client
        .virtual_fields
        .list(ListOptions {
            dataset: ctx.dataset_id.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(virtual_fields.contains(&ctx.virtual_field));
}
