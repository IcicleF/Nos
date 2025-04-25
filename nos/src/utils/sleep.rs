//! Asynchronous sleep similar to `tokio::time::sleep`, but without the need for a runtime.

use quanta::Instant;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// Future that resolves after a specified duration.
#[derive(Debug)]
struct Sleep {
    until: Instant,
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        if Instant::now() >= self.until {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// Sleep for the specified duration.
pub fn async_sleep(duration: Duration) -> impl Future<Output = ()> {
    Sleep {
        until: Instant::now() + duration,
    }
}
