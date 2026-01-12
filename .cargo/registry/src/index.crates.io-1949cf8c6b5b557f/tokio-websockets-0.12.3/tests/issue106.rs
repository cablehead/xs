#![cfg(feature = "client")]

use std::{
    io::Result,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake},
};

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_websockets::{ClientBuilder, Message};

struct CountWaker(AtomicUsize);

impl Wake for CountWaker {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

struct MockStream;

impl AsyncRead for MockStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockStream {
    // We can track the amount of flushes called on the WebSocketStream by counting
    // the writes to the mock stream, since flushing the WebSocketStream writes
    // pending data to the mock stream.
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, _: &[u8]) -> Poll<Result<usize>> {
        cx.waker().wake_by_ref();
        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Pending
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Pending
    }
}

/// Test that calling poll_next always attempts to flush the stream.
#[test]
fn main() {
    let mut ws = ClientBuilder::new().take_over(MockStream);

    // queue a message to trigger a flush on next read
    let _ = ws.start_send_unpin(Message::text("message"));
    let count1 = Arc::new(CountWaker(AtomicUsize::new(0)));
    let count2 = Arc::new(CountWaker(AtomicUsize::new(0)));
    let waker1 = count1.clone().into();
    let waker2 = count2.clone().into();

    // queued message triggers a flush
    let _ = ws.poll_flush_unpin(&mut Context::from_waker(&waker1));

    // there's still messages queued, last flush's waker is reused
    let _ = ws.poll_next_unpin(&mut Context::from_waker(&waker2));

    assert_eq!(count1.0.load(Ordering::Relaxed), 2);
    assert_eq!(count2.0.load(Ordering::Relaxed), 0);
}
