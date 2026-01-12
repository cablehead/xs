use surge_ping::{Client, Config, PingIdentifier, PingSequence, SurgeError};

#[tokio::test]
async fn test_pinger_after_client_destroyed() {
    let config = Config::default();
    let client = Client::new(&config).unwrap();

    // Create a pinger while client is alive
    let mut pinger = client
        .pinger("8.8.8.8".parse().unwrap(), PingIdentifier(42))
        .await;

    // Drop the client
    drop(client);

    // Give some time for drop to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Now try to ping - should get ClientDestroyed error instead of timeout
    let result = pinger.ping(PingSequence(0), &[0; 8]).await;

    match result {
        Err(SurgeError::ClientDestroyed) => {
            // This is the expected behavior
        }
        Err(other) => {
            panic!("Expected ClientDestroyed error, got: {:?}", other);
        }
        Ok(_) => {
            panic!("Expected ClientDestroyed error, got Ok result");
        }
    }
}
