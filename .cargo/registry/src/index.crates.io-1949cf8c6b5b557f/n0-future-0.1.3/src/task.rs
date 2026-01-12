//! Async rust task spawning and utilities that work natively (using tokio) and in browsers
//! (using wasm-bindgen-futures).

#[cfg(not(wasm_browser))]
pub use tokio::spawn;
#[cfg(not(wasm_browser))]
pub use tokio::task::{JoinError, JoinHandle, JoinSet};
#[cfg(not(wasm_browser))]
pub use tokio_util::task::AbortOnDropHandle;

#[cfg(wasm_browser)]
pub use wasm::*;

#[cfg(wasm_browser)]
mod wasm {
    use std::{
        cell::RefCell,
        fmt::Debug,
        future::{Future, IntoFuture},
        pin::Pin,
        rc::Rc,
        task::{Context, Poll, Waker},
    };

    use futures_lite::stream::StreamExt;
    use send_wrapper::SendWrapper;

    /// Wasm shim for tokio's `JoinSet`.
    ///
    /// Uses a `futures_buffered::FuturesUnordered` queue of
    /// `JoinHandle`s inside.
    pub struct JoinSet<T> {
        handles: futures_buffered::FuturesUnordered<JoinHandle<T>>,
        // We need to keep a second list of JoinHandles so we can access them for cancellation
        to_cancel: Vec<JoinHandle<T>>,
    }

    impl<T> std::fmt::Debug for JoinSet<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("JoinSet").field("len", &self.len()).finish()
        }
    }

    impl<T> Default for JoinSet<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T> JoinSet<T> {
        /// Creates a new, empty `JoinSet`
        pub fn new() -> Self {
            Self {
                handles: futures_buffered::FuturesUnordered::new(),
                to_cancel: Vec::new(),
            }
        }

        /// Spawns a task into this `JoinSet`.
        ///
        /// (Doesn't return an `AbortHandle` unlike the original `tokio::task::JoinSet` yet.)
        pub fn spawn(&mut self, fut: impl IntoFuture<Output = T> + 'static)
        where
            T: 'static,
        {
            let handle = JoinHandle::new();
            let handle_for_spawn = JoinHandle {
                task: handle.task.clone(),
            };
            let handle_for_cancel = JoinHandle {
                task: handle.task.clone(),
            };

            wasm_bindgen_futures::spawn_local(SpawnFuture {
                handle: handle_for_spawn,
                fut: fut.into_future(),
            });

            self.handles.push(handle);
            self.to_cancel.push(handle_for_cancel);
        }

        /// Aborts all tasks inside this `JoinSet`
        pub fn abort_all(&self) {
            self.to_cancel.iter().for_each(JoinHandle::abort);
        }

        /// Awaits the next `JoinSet`'s completion.
        ///
        /// If you `.spawn` a new task onto this `JoinSet` while the future
        /// returned from this is currently pending, then this future will
        /// continue to be pending, even if the newly spawned future is already
        /// finished.
        ///
        /// TODO(matheus23): Fix this limitation.
        ///
        /// Current work around is to recreate the `join_next` future when
        /// you newly spawned a task onto it. This seems to be the usual way
        /// the `JoinSet` is used *most of the time* in the iroh codebase anyways.
        pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
            futures_lite::future::poll_fn(|cx| {
                let ret = self.handles.poll_next(cx);
                // clean up handles that are either cancelled or have finished
                self.to_cancel.retain(JoinHandle::is_running);
                ret
            })
            .await
        }

        /// Returns whether there's any tasks that are either still running or
        /// have pending results in this `JoinSet`.
        pub fn is_empty(&self) -> bool {
            self.handles.is_empty()
        }

        /// Returns the amount of tasks that are either still running or have
        /// pending results in this `JoinSet`.
        pub fn len(&self) -> usize {
            self.handles.len()
        }

        /// Waits for all tasks to finish. If any of them returns a JoinError,
        /// this will panic.
        pub async fn join_all(mut self) -> Vec<T> {
            let mut output = Vec::new();
            while let Some(res) = self.join_next().await {
                match res {
                    Ok(t) => output.push(t),
                    Err(err) => panic!("{err}"),
                }
            }
            output
        }

        /// Aborts all tasks and then waits for them to finish, ignoring panics.
        pub async fn shutdown(&mut self) {
            self.abort_all();
            while let Some(_res) = self.join_next().await {}
        }
    }

    impl<T> Drop for JoinSet<T> {
        fn drop(&mut self) {
            self.abort_all()
        }
    }

    /// A handle to a spawned task.
    pub struct JoinHandle<T> {
        // Using SendWrapper here is safe as long as you keep all of your
        // work on the main UI worker in the browser.
        // The only exception to that being the case would be if our user
        // would use multiple Wasm instances with a single SharedArrayBuffer,
        // put the instances on different Web Workers and finally shared
        // the JoinHandle across the Web Worker boundary.
        // In that case, using the JoinHandle would panic.
        task: SendWrapper<Rc<RefCell<Task<T>>>>,
    }

    struct Task<T> {
        cancelled: bool,
        completed: bool,
        waker_handler: Option<Waker>,
        waker_spawn_fn: Option<Waker>,
        result: Option<T>,
    }

    impl<T> Task<T> {
        fn cancel(&mut self) {
            if !self.cancelled {
                self.cancelled = true;
                self.wake();
            }
        }

        fn complete(&mut self, value: T) {
            self.result = Some(value);
            self.completed = true;
            self.wake();
        }

        fn wake(&mut self) {
            if let Some(waker) = self.waker_handler.take() {
                waker.wake();
            }
            if let Some(waker) = self.waker_spawn_fn.take() {
                waker.wake();
            }
        }

        fn register_handler(&mut self, cx: &mut Context<'_>) {
            match self.waker_handler {
                // clone_from can be marginally faster in some cases
                Some(ref mut waker) => waker.clone_from(cx.waker()),
                None => self.waker_handler = Some(cx.waker().clone()),
            }
        }

        fn register_spawn_fn(&mut self, cx: &mut Context<'_>) {
            match self.waker_spawn_fn {
                // clone_from can be marginally faster in some cases
                Some(ref mut waker) => waker.clone_from(cx.waker()),
                None => self.waker_spawn_fn = Some(cx.waker().clone()),
            }
        }
    }

    impl<T> Debug for JoinHandle<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if self.task.valid() {
                let task = self.task.borrow();
                let cancelled = task.cancelled;
                let completed = task.completed;
                f.debug_struct("JoinHandle")
                    .field("cancelled", &cancelled)
                    .field("completed", &completed)
                    .finish()
            } else {
                f.debug_tuple("JoinHandle")
                    .field(&format_args!("<other thread>"))
                    .finish()
            }
        }
    }

    impl<T> JoinHandle<T> {
        fn new() -> Self {
            Self {
                task: SendWrapper::new(Rc::new(RefCell::new(Task {
                    cancelled: false,
                    completed: false,
                    waker_handler: None,
                    waker_spawn_fn: None,
                    result: None,
                }))),
            }
        }

        /// Aborts this task.
        pub fn abort(&self) {
            self.task.borrow_mut().cancel();
        }

        fn is_running(&self) -> bool {
            let task = self.task.borrow();
            !task.cancelled && !task.completed
        }
    }

    /// An error that can occur when waiting for the completion of a task.
    #[derive(derive_more::Display, Debug, Clone, Copy)]
    pub enum JoinError {
        /// The error that's returned when the task that's being waited on
        /// has been cancelled.
        #[display("task was cancelled")]
        Cancelled,
    }

    impl std::error::Error for JoinError {}

    impl JoinError {
        /// Returns whether this join error is due to cancellation.
        ///
        /// Always true in this Wasm implementation, because we don't
        /// unwind panics in tasks.
        /// All panics just happen on the main thread anyways.
        pub fn is_cancelled(&self) -> bool {
            matches!(self, Self::Cancelled)
        }

        /// Returns whether this is a panic. Always `false` in Wasm,
        /// because when a task panics, it's not unwound, instead it
        /// panics directly to the main thread.
        pub fn is_panic(&self) -> bool {
            false
        }
    }

    impl<T> Future for JoinHandle<T> {
        type Output = Result<T, JoinError>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut task = self.task.borrow_mut();
            if task.cancelled {
                return Poll::Ready(Err(JoinError::Cancelled));
            }

            if let Some(result) = task.result.take() {
                return Poll::Ready(Ok(result));
            }

            task.register_handler(cx);
            Poll::Pending
        }
    }

    #[pin_project::pin_project]
    struct SpawnFuture<Fut: Future<Output = T>, T> {
        handle: JoinHandle<T>,
        #[pin]
        fut: Fut,
    }

    impl<Fut: Future<Output = T>, T> Future for SpawnFuture<Fut, T> {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            let mut task = this.handle.task.borrow_mut();

            if task.cancelled {
                return Poll::Ready(());
            }

            match this.fut.poll(cx) {
                Poll::Ready(value) => {
                    task.complete(value);
                    Poll::Ready(())
                }
                Poll::Pending => {
                    task.register_spawn_fn(cx);
                    Poll::Pending
                }
            }
        }
    }

    /// Similar to a `JoinHandle`, except it automatically aborts
    /// the task when it's dropped.
    #[pin_project::pin_project(PinnedDrop)]
    #[derive(derive_more::Debug)]
    #[debug("AbortOnDropHandle")]
    pub struct AbortOnDropHandle<T>(#[pin] JoinHandle<T>);

    #[pin_project::pinned_drop]
    impl<T> PinnedDrop for AbortOnDropHandle<T> {
        fn drop(self: Pin<&mut Self>) {
            self.0.abort();
        }
    }

    impl<T> Future for AbortOnDropHandle<T> {
        type Output = <JoinHandle<T> as Future>::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.project().0.poll(cx)
        }
    }

    impl<T> AbortOnDropHandle<T> {
        /// Converts a `JoinHandle` into one that aborts on drop.
        pub fn new(task: JoinHandle<T>) -> Self {
            Self(task)
        }
    }

    /// Spawns a future as a task in the browser runtime.
    ///
    /// This is powered by `wasm_bidngen_futures`.
    pub fn spawn<T: 'static>(fut: impl IntoFuture<Output = T> + 'static) -> JoinHandle<T> {
        let handle = JoinHandle::new();

        wasm_bindgen_futures::spawn_local(SpawnFuture {
            handle: JoinHandle {
                task: handle.task.clone(),
            },
            fut: fut.into_future(),
        });

        handle
    }
}

#[cfg(test)]
mod test {
    // TODO(matheus23): Test wasm shims using wasm-bindgen-test
}
