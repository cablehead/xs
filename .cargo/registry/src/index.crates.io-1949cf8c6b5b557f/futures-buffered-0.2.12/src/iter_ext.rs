use core::future::Future;

use futures_core::Stream;

use crate::{
    join_all, try_join_all, FuturesOrdered, FuturesOrderedBounded, FuturesUnordered,
    FuturesUnorderedBounded, JoinAll, MergeBounded, MergeUnbounded, TryFuture, TryJoinAll,
};

/// Concurrency extensions for iterators of streams and futures.
pub trait IterExt: IntoIterator {
    /// Combines an iterator of streams into a single stream, yielding items as they arrive.
    ///
    /// Returns [`MergeBounded`], which has a fixed capacity and thus no further streams may be added.
    ///
    /// ## Example
    /// ```
    /// use futures::{stream, StreamExt};
    /// use futures_buffered::IterExt;
    ///
    /// # #[tokio::main] async fn main() {
    /// let res = [stream::iter(0..3), stream::iter(0..5)]
    ///     .merge()
    ///     .count()
    ///     .await;
    /// assert_eq!(res, 3 + 5);
    /// # }
    /// ```
    fn merge(self) -> MergeBounded<Self::Item>
    where
        Self: Sized,
        Self::Item: Stream,
    {
        MergeBounded::from_iter(self)
    }

    /// Combines an iterator of streams into a single stream, yielding items as they arrive.
    ///
    /// This is like [`IterExt::merge`], but  returns [`MergeUnbounded`], to which further streams
    /// may be added with [`MergeUnbounded::push`]. If you don't need to add more streams, use
    /// [`IterExt::merge`], which has better performance characteristics.
    fn merge_unbounded(self) -> MergeUnbounded<Self::Item>
    where
        Self: Sized,
        Self::Item: Stream + Unpin,
    {
        MergeUnbounded::from_iter(self)
    }

    /// Waits for all futures to complete, returning a `Vec` of their outputs.
    ///
    /// All futures are driven concurrently to completion, and their results are
    /// collected into a `Vec` in same order as they were provided.
    ///
    /// See [`join_all`] for details.
    ///
    /// ## Example
    /// ```
    /// use futures_buffered::IterExt;
    /// # #[tokio::main] async fn main() {
    /// let res: Vec<_> = [3, 2, 1]
    ///     .map(|x| async move { x })
    ///     .join_all()
    ///     .await;
    /// assert_eq!(res, vec![3, 2, 1]);
    /// # }
    /// ```
    fn join_all(self) -> JoinAll<Self::Item>
    where
        Self: Sized,
        Self::Item: Future,
    {
        join_all(self)
    }

    /// Waits for all futures to complete, returning a `Result<Vec<T>, E>`.
    ///
    /// If any future returns an error then all other futures will be canceled and
    /// the error will be returned immediately. If all futures complete successfully,
    /// then the returned future will succeed with a `Vec` of all the successful
    /// results in the same order as the futures were provided.
    ///
    /// See [`try_join_all`] for details.
    fn try_join_all(self) -> TryJoinAll<Self::Item>
    where
        Self: Sized,
        Self::Item: TryFuture,
    {
        try_join_all(self)
    }

    /// Combines an iterator of futures into a concurrent stream, yielding items as they arrive.
    ///
    /// The futures are polled concurrently and items are yielded in the order of completion.
    ///
    /// Returns [`FuturesUnorderedBounded`], which has a fixed capacity so no further futures can be
    /// added to the stream.
    ///
    /// ## Example
    /// ```
    /// use futures::StreamExt;
    /// use futures_buffered::IterExt;
    /// use tokio::time::{sleep, Duration};
    ///
    /// # #[cfg(miri)] fn main() {}
    /// # #[cfg(not(miri))] #[tokio::main]
    /// # async fn main() {
    /// let res: Vec<_> = [3, 2, 1]
    ///     .map(|x| async move {
    ///         sleep(Duration::from_millis(x * 10)).await;
    ///         x
    ///     })
    ///     .into_unordered_stream()
    ///     .collect()
    ///     .await;
    /// assert_eq!(res, vec![1, 2, 3]);
    /// # }
    /// ```
    fn into_unordered_stream(self) -> FuturesUnorderedBounded<Self::Item>
    where
        Self: Sized,
        Self::Item: Future,
    {
        FuturesUnorderedBounded::from_iter(self)
    }

    /// Combines an iterator of futures into a concurrent stream, yielding items as they arrive.
    ///
    /// The futures are polled concurrently and items are yielded in the order of completion.
    ///
    /// Returns [`FuturesUnordered`], which can grow capacity on demand, so further futures can be
    /// added to the stream via [`FuturesUnordered::push`].
    fn into_unordered_stream_unbounded(self) -> FuturesUnordered<Self::Item>
    where
        Self: Sized,
        Self::Item: Future,
    {
        FuturesUnordered::from_iter(self)
    }

    /// Combines an iterator of futures into a concurrent stream, yielding items in their original order.
    ///
    /// The futures are polled concurrently and items are yielded in the order of the source iterator.
    ///
    /// Returns [`FuturesOrderedBounded`], which has a fixed capacity so no further futures can be
    /// added to the stream.
    ///
    /// ## Example
    /// ```
    /// use futures::StreamExt;
    /// use futures_buffered::IterExt;
    /// use tokio::time::{sleep, Duration};
    ///
    /// # #[cfg(miri)] fn main() {}
    /// # #[cfg(not(miri))] #[tokio::main]
    /// # async fn main() {
    /// let res: Vec<_> = [3, 2, 1]
    ///     .map(|x| async move {
    ///         sleep(Duration::from_millis(x * 10)).await;
    ///         x
    ///     })
    ///     .into_ordered_stream()
    ///     .collect()
    ///     .await;
    /// assert_eq!(res, vec![3, 2, 1]);
    /// # }
    /// ```
    fn into_ordered_stream(self) -> FuturesOrderedBounded<Self::Item>
    where
        Self: Sized,
        Self::Item: Future,
    {
        FuturesOrderedBounded::from_iter(self)
    }

    /// Combines an iterator of futures into a concurrent stream, yielding items in their original order.
    ///
    /// The futures are polled concurrently and items are yielded in the order of the source iterator.
    ///
    /// Returns [`FuturesOrdered`], which can grow capacity on demand, so further futures can be
    /// added to the stream via [`FuturesOrdered::push_back`] or [`FuturesOrdered::push_front`].
    fn into_ordered_stream_unbounded(self) -> FuturesOrdered<Self::Item>
    where
        Self: Sized,
        Self::Item: Future,
    {
        FuturesOrdered::from_iter(self)
    }
}

impl<T: IntoIterator> IterExt for T {}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::vec::Vec;

    use futures::{FutureExt, StreamExt};

    use super::IterExt;

    #[cfg(not(miri))]
    #[tokio::test]
    async fn smoke() {
        let to_future = |x: u64| async move {
            tokio::time::sleep(Duration::from_millis(x * 10)).await;
            x
        };

        let res: Vec<_> = [3, 2, 1]
            .map(to_future)
            .into_ordered_stream()
            .collect()
            .await;
        assert_eq!(res, vec![3, 2, 1]);

        let res: Vec<_> = [3, 2, 1]
            .map(to_future)
            .into_unordered_stream()
            .collect()
            .await;
        assert_eq!(res, vec![1, 2, 3]);

        let res: Vec<_> = [3, 2, 1]
            .map(to_future)
            .into_ordered_stream_unbounded()
            .collect()
            .await;
        assert_eq!(res, vec![3, 2, 1]);

        let res: Vec<_> = [3, 2, 1]
            .map(to_future)
            .into_unordered_stream_unbounded()
            .collect()
            .await;
        assert_eq!(res, vec![1, 2, 3]);

        let res: Vec<_> = [3, 2, 1].map(to_future).join_all().await;
        assert_eq!(res, vec![3, 2, 1]);

        let res: Result<Vec<_>, ()> = [3, 2, 1]
            .map(|x| to_future(x).map(Result::Ok))
            .try_join_all()
            .await;
        assert_eq!(res, Ok(vec![3, 2, 1]));

        let res = [3, 2, 1]
            .map(|x| to_future(x).map(|x| if x == 2 { Err(x) } else { Ok(x) }))
            .try_join_all()
            .await;
        assert_eq!(res, Err(2));

        let res = [3, 2, 1]
            .map(|x| futures::stream::iter(0..x))
            .merge()
            .count()
            .await;
        assert_eq!(res, 3 + 2 + 1);

        let res = [3, 2, 1]
            .map(|x| futures::stream::iter(0..x))
            .merge_unbounded()
            .count()
            .await;
        assert_eq!(res, 3 + 2 + 1);
    }
}
