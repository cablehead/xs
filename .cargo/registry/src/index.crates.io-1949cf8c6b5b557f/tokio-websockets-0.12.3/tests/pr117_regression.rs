#![cfg(all(feature = "client", feature = "server"))]

use futures_util::{stream, SinkExt, StreamExt};
use tokio::io::{duplex, AsyncRead, AsyncWrite};
use tokio_websockets::{ClientBuilder, Message, ServerBuilder};

const NUM_MSG: usize = 1024;
const MESSAGE: &str = "test message";

#[tokio::test]
async fn test_pr117_regression() {
    let (tx, rx) = duplex(64);

    tokio::join!(server(rx), client(tx));
}

async fn server<S>(stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let server = ServerBuilder::new().serve(stream);

    let messages = server
        .inspect(|message| {
            let message = message.as_ref().unwrap();
            assert!(matches!(message.as_text(), Some(MESSAGE)));
        })
        .collect::<Vec<_>>()
        .await;

    assert_eq!(messages.len(), NUM_MSG);
}

async fn client<S>(stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut client = ClientBuilder::new().take_over(stream);
    let mut messages = stream::iter((0..NUM_MSG).map(|_| Ok(Message::text(MESSAGE))));
    client.send_all(&mut messages).await.unwrap();
}
