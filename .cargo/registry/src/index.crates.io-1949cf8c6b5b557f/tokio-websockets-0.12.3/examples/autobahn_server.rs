use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{Error, Limits, ServerBuilder};

async fn accept_connection(stream: TcpStream) {
    if let Err(e) = handle_connection(stream).await {
        match e {
            Error::Protocol(_) => (),
            err => eprintln!("Error processing connection: {err:?}"),
        }
    }
}

async fn handle_connection(stream: TcpStream) -> Result<(), Error> {
    let (_request, mut ws_stream) = ServerBuilder::new()
        .limits(Limits::unlimited())
        .accept(stream)
        .await?;

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;

        if msg.is_text() || msg.is_binary() {
            ws_stream.send(msg).await?;
        }
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let addr: SocketAddr = ([127, 0, 0, 1], 9006).into();
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");

    println!("Listening on: {addr}");

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }
}
