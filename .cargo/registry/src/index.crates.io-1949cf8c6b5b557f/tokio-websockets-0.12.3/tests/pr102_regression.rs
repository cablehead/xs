#![cfg(all(feature = "client", feature = "server"))]
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::{io::duplex, time::timeout};
use tokio_websockets::{ClientBuilder, Message, ServerBuilder};

#[tokio::test]
async fn test_pr102_regression() {
    let (tx, rx) = duplex(64);

    // Create a server that sends a close message and immediately closes the
    // connection
    let mut server = ServerBuilder::new().serve(rx);
    // Don't use ws.close() here to avoid a graceful handshake
    server.send(Message::close(None, "")).await.unwrap();
    // This will close the connection
    drop(server);

    // Create a client and make sure the close frame from the server was sent and
    // the connection closed
    let mut client = ClientBuilder::new().take_over(tx);

    // Receive the close frame, this will queue the close ack frame
    assert!(client.next().await.unwrap().unwrap().is_close());
    // The close ack frame is queued, the connection is closed, before PR 102 this
    // would trigger an infinite loop in close()
    // For the regression test, fail the test if this doesn't close within a second
    assert!(timeout(Duration::from_secs(1), client.close())
        .await
        .is_ok());
}
