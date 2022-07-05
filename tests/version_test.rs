use axiom_rs::Client;

#[tokio::test]
async fn test_version() {
    let client = Client::new().unwrap();
    let version = client.version().await.unwrap();
    assert!(version.server.starts_with("v"));
    assert_eq!(env!("CARGO_PKG_VERSION"), version.client);
}
