//! Sleep and timeout utilities that work natively (via tokio) and in the browser.

#[cfg(not(wasm_browser))]
pub use std::time::SystemTime;
#[cfg(not(wasm_browser))]
pub use tokio::time::{
    error::Elapsed, interval, interval_at, sleep, sleep_until, timeout, Duration, Instant,
    Interval, MissedTickBehavior, Sleep, Timeout,
};

#[cfg(wasm_browser)]
pub use wasm::{
    error::Elapsed, interval, interval_at, sleep, sleep_until, timeout, Duration, Instant,
    Interval, MissedTickBehavior, Sleep, SystemTime, Timeout,
};

#[cfg(wasm_browser)]
mod wasm {
    use futures_util::task::AtomicWaker;
    use send_wrapper::SendWrapper;
    use std::{
        future::{Future, IntoFuture},
        pin::Pin,
        sync::{
            atomic::{AtomicBool, Ordering::Relaxed},
            Arc,
        },
        task::{Context, Poll},
    };
    use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue};

    pub use web_time::{Duration, Instant, SystemTime};

    /// Future that will wake up once its deadline is reached.
    #[derive(Debug)]
    pub struct Sleep {
        deadline: Instant,
        triggered: Option<Flag>,
        timeout_id: Option<SendWrapper<JsValue>>,
    }

    /// Sleeps for given duration
    pub fn sleep(duration: Duration) -> Sleep {
        // javascript can't handle setTimeout durations as big as rust, so we
        // can't rely on `now.checked_add` to overflow.
        if duration > Duration::from_secs(60 * 60 * 24 * 365 * 10) {
            return sleep_forever();
        }
        let now = Instant::now();
        if let Some(deadline) = now.checked_add(duration) {
            sleep_impl(duration, deadline)
        } else {
            sleep_forever()
        }
    }

    /// Sleeps until given deadline
    pub fn sleep_until(deadline: Instant) -> Sleep {
        let now = Instant::now();
        let duration = deadline.duration_since(now);
        sleep_impl(duration, deadline)
    }

    fn sleep_impl(duration: Duration, deadline: Instant) -> Sleep {
        let triggered = Flag::new();

        let closure = Closure::once({
            let triggered = triggered.clone();
            move || triggered.signal()
        });

        let timeout_id = Some(SendWrapper::new(
            set_timeout(
                closure.into_js_value().unchecked_into(),
                duration.as_millis() as i32,
            )
            .expect("missing setTimeout function on globalThis"),
        ));

        Sleep {
            deadline,
            triggered: Some(triggered),
            timeout_id,
        }
    }

    fn sleep_forever() -> Sleep {
        // fake a deadline that's far in the future (10 years)
        let deadline = Instant::now() + Duration::from_secs(60 * 60 * 24 * 365 * 10);
        Sleep {
            triggered: None,
            deadline,
            timeout_id: None,
        }
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match &mut self.triggered {
                Some(ref mut triggered) => Pin::new(triggered).poll_signaled(cx),
                None => Poll::Pending,
            }
        }
    }

    impl Drop for Sleep {
        fn drop(&mut self) {
            if let Some(timeout_id) = self.timeout_id.as_ref() {
                // If not, then in the worst case we're leaking a timeout
                if timeout_id.valid() {
                    clear_timeout(timeout_id.as_ref().clone()).ok();
                }
            }
        }
    }

    impl Sleep {
        /// Returns the instant at which the sleep is scheduled to wake up
        pub fn deadline(&self) -> Instant {
            self.deadline
        }

        /// Returns whether the sleep has reached its deadline
        /// (and the scheduler has handled the sleep's timer).
        pub fn is_elapsed(&self) -> bool {
            self.triggered.as_ref().map_or(false, Flag::has_triggered)
        }

        /// Resets this sleep's deadline to given instant.
        ///
        /// Also works with sleeps that have already reached their deadline
        /// in the past.
        pub fn reset(mut self: Pin<&mut Self>, deadline: Instant) {
            let duration = deadline.duration_since(Instant::now());
            let triggered = Flag::new();

            let closure = Closure::once({
                let triggered = triggered.clone();
                move || triggered.signal()
            });

            let timeout_id = SendWrapper::new(
                set_timeout(
                    closure.into_js_value().unchecked_into(),
                    duration.as_millis() as i32,
                )
                .expect("missing setTimeout function on globalThis"),
            );

            let mut this = self.as_mut();
            this.deadline = deadline;
            this.triggered = Some(triggered);
            let old_timeout_id = std::mem::replace(&mut this.timeout_id, Some(timeout_id));
            if let Some(timeout_id) = old_timeout_id {
                // If not valid, then in the worst case we're leaking a timeout task
                if timeout_id.valid() {
                    clear_timeout(timeout_id.as_ref().clone()).ok();
                }
            }
        }

        /// Resets this sleep to never wake up again (unless reset to a different timeout).
        fn reset_forever(mut self: Pin<&mut Self>) {
            let mut this = self.as_mut();
            this.deadline = Instant::now() + Duration::from_secs(60 * 60 * 24 * 365 * 10);
            this.triggered = None;
            let old_timeout_id = std::mem::replace(&mut this.timeout_id, None);
            if let Some(timeout_id) = old_timeout_id {
                // If not valid, then in the worst case we're leaking a timeout task
                if timeout_id.valid() {
                    clear_timeout(timeout_id.as_ref().clone()).ok();
                }
            }
        }
    }

    /// Future that either resolves to [`error::Elapsed`] if the timeout
    /// is hit first. Otherwise, it resolves to `Ok` of the wrapped future.
    #[derive(Debug)]
    #[pin_project::pin_project]
    pub struct Timeout<T> {
        #[pin]
        future: T,
        #[pin]
        sleep: Sleep,
    }

    /// Error structs for time utilities (wasm mirror for `tokio::time::error`).
    pub mod error {
        /// Error when a timeout is elapsed.
        #[derive(Debug, derive_more::Display)]
        #[display("deadline has elapsed")]
        pub struct Elapsed;

        impl std::error::Error for Elapsed {}
    }

    /// Timeout of a function in wasm.
    pub fn timeout<F>(duration: Duration, future: F) -> Timeout<F::IntoFuture>
    where
        F: IntoFuture,
    {
        Timeout {
            future: future.into_future(),
            sleep: sleep(duration),
        }
    }

    impl<T: Future> Future for Timeout<T> {
        type Output = Result<T::Output, error::Elapsed>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();

            if let Poll::Ready(result) = this.future.poll(cx) {
                return Poll::Ready(Ok(result));
            }

            if let Poll::Ready(()) = this.sleep.poll(cx) {
                return Poll::Ready(Err(error::Elapsed));
            }

            Poll::Pending
        }
    }

    impl<T> Timeout<T> {
        /// Returns a reference of the wrapped future.
        pub fn get_ref(&self) -> &T {
            &self.future
        }

        /// Returns a mutable reference to the wrapped future.
        pub fn get_mut(&mut self) -> &mut T {
            &mut self.future
        }

        /// Returns the wrapped future and throws away and cancels the
        /// associated timeout.
        pub fn into_inner(self) -> T {
            self.future
        }
    }

    /// Defines the behavior of an [`Interval`] when it misses a tick.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum MissedTickBehavior {
        /// Ticks as fast as possible until caught up.
        #[default]
        Burst,

        /// Tick at multiples of `period` from when [`tick`] was called, rather than
        /// from `start`.
        Delay,

        /// Skips missed ticks and tick on the next multiple of `period` from
        /// `start`.
        Skip,
    }

    impl MissedTickBehavior {
        /// If a tick is missed, this method is called to determine when the next tick should happen.
        fn next_timeout(&self, timeout: Instant, now: Instant, period: Duration) -> Instant {
            match self {
                Self::Burst => timeout + period,
                Self::Delay => now + period,
                Self::Skip => {
                    now + period
                    - Duration::from_nanos(
                        ((now - timeout).as_nanos() % period.as_nanos())
                            .try_into()
                            // This operation is practically guaranteed not to
                            // fail, as in order for it to fail, `period` would
                            // have to be longer than `now - timeout`, and both
                            // would have to be longer than 584 years.
                            //
                            // If it did fail, there's not a good way to pass
                            // the error along to the user, so we just panic.
                            .expect(
                                "too much time has elapsed since the interval was supposed to tick",
                            ),
                    )
                }
            }
        }
    }

    /// Interval returned by [`interval`] and [`interval_at`].
    #[derive(Debug)]
    pub struct Interval {
        delay: Pin<Box<Sleep>>,
        period: Duration,
        missed_tick_behavior: MissedTickBehavior,
    }

    /// Creates new [`Interval`] that yields with interval of `period`. The first
    /// tick completes immediately. The default [`MissedTickBehavior`] is
    /// [`Burst`](MissedTickBehavior::Burst), but this can be configured
    /// by calling [`set_missed_tick_behavior`](Interval::set_missed_tick_behavior).
    ///
    /// An interval will tick indefinitely. At any time, the [`Interval`] value can
    /// be dropped. This cancels the interval.
    ///
    /// This function is equivalent to
    /// [`interval_at(Instant::now(), period)`](interval_at).
    pub fn interval(period: Duration) -> Interval {
        assert!(period > Duration::new(0, 0), "`period` must be non-zero.");

        interval_at(Instant::now(), period)
    }

    /// Creates new [`Interval`] that yields with interval of `period` with the
    /// first tick completing at `start`. The default [`MissedTickBehavior`] is
    /// [`Burst`](MissedTickBehavior::Burst), but this can be configured
    /// by calling [`set_missed_tick_behavior`](Interval::set_missed_tick_behavior).
    ///
    /// An interval will tick indefinitely. At any time, the [`Interval`] value can
    /// be dropped. This cancels the interval.
    #[track_caller]
    pub fn interval_at(start: Instant, period: Duration) -> Interval {
        assert!(period > Duration::new(0, 0), "`period` must be non-zero.");

        let delay = Box::pin(sleep_until(start));

        Interval {
            delay,
            period,
            missed_tick_behavior: MissedTickBehavior::default(),
        }
    }

    impl Interval {
        /// Completes when the next instant in the interval has been reached.
        pub async fn tick(&mut self) -> Instant {
            futures_lite::future::poll_fn(|cx| self.poll_tick(cx)).await
        }

        /// Polls for the next instant in the interval to be reached.
        pub fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Instant> {
            // Wait for the delay to be done
            futures_lite::ready!(Pin::new(&mut self.delay).poll(cx));

            // Get the time when we were scheduled to tick
            let timeout = self.delay.deadline();

            let now = Instant::now();

            // If a tick was not missed, and thus we are being called before the
            // next tick is due, just schedule the next tick normally, one `period`
            // after `timeout`
            //
            // However, if a tick took excessively long and we are now behind,
            // schedule the next tick according to how the user specified with
            // `MissedTickBehavior`
            let next = if now > timeout + Duration::from_millis(5) {
                Some(
                    self.missed_tick_behavior
                        .next_timeout(timeout, now, self.period),
                )
            } else {
                timeout.checked_add(self.period)
            };

            if let Some(next) = next {
                self.delay.as_mut().reset(next);
            } else {
                self.delay.as_mut().reset_forever()
            }

            // Return the time when we were scheduled to tick
            Poll::Ready(timeout)
        }

        /// Resets the interval to complete one period after the current time.
        pub fn reset(&mut self) {
            self.delay.as_mut().reset(Instant::now() + self.period);
        }

        /// Resets the interval immediately.
        pub fn reset_immediately(&mut self) {
            self.delay.as_mut().reset(Instant::now());
        }

        /// Resets the interval after the specified [`std::time::Duration`].
        pub fn reset_after(&mut self, after: Duration) {
            self.delay.as_mut().reset(Instant::now() + after);
        }

        /// Resets the interval to a [`crate::time::Instant`] deadline.
        pub fn reset_at(&mut self, deadline: Instant) {
            self.delay.as_mut().reset(deadline);
        }

        /// Returns the [`MissedTickBehavior`] strategy currently being used.
        pub fn missed_tick_behavior(&self) -> MissedTickBehavior {
            self.missed_tick_behavior
        }

        /// Sets the [`MissedTickBehavior`] strategy that should be used.
        pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior) {
            self.missed_tick_behavior = behavior;
        }

        /// Returns the period of the interval.
        pub fn period(&self) -> Duration {
            self.period
        }
    }

    // Private impls

    #[derive(Clone, Debug)]
    struct Flag(Arc<Inner>);

    #[derive(Debug)]
    struct Inner {
        waker: AtomicWaker,
        set: AtomicBool,
    }

    impl Flag {
        fn new() -> Self {
            Self(Arc::new(Inner {
                waker: AtomicWaker::new(),
                set: AtomicBool::new(false),
            }))
        }

        fn has_triggered(&self) -> bool {
            self.0.set.load(Relaxed)
        }

        fn signal(&self) {
            self.0.set.store(true, Relaxed);
            self.0.waker.wake();
        }

        fn poll_signaled(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            // quick check to avoid registration if already done.
            if self.0.set.load(Relaxed) {
                return Poll::Ready(());
            }

            self.0.waker.register(cx.waker());

            // Need to check condition **after** `register` to avoid a race
            // condition that would result in lost notifications.
            if self.0.set.load(Relaxed) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }

    // Wasm-bindgen stuff

    #[wasm_bindgen]
    extern "C" {
        type GlobalScope;

        #[wasm_bindgen(catch, method, js_name = "setTimeout")]
        fn set_timeout_with_callback_and_timeout_and_arguments_0(
            this: &GlobalScope,
            handler: js_sys::Function,
            timeout: i32,
        ) -> Result<JsValue, JsValue>;

        #[wasm_bindgen(catch, method, js_name = "clearTimeout")]
        fn clear_timeout_with_handle(
            this: &GlobalScope,
            timeout_id: JsValue,
        ) -> Result<(), JsValue>;
    }

    fn set_timeout(handler: js_sys::Function, timeout: i32) -> Result<JsValue, JsValue> {
        let global_this = js_sys::global();
        let global_scope = global_this.unchecked_ref::<GlobalScope>();
        global_scope.set_timeout_with_callback_and_timeout_and_arguments_0(handler, timeout)
    }

    fn clear_timeout(timeout_id: JsValue) -> Result<(), JsValue> {
        let global_this = js_sys::global();
        let global_scope = global_this.unchecked_ref::<GlobalScope>();
        global_scope.clear_timeout_with_handle(timeout_id)
    }
}

#[cfg(test)]
mod tests {
    // TODO(matheus23): Write some tests for `sleep`, `sleep_until`, `timeout` and `Sleep::reset`
}
