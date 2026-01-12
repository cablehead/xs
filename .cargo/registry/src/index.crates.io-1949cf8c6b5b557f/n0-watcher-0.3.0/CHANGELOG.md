# v0.3.0 - 2025-07-24

Features:
- Added `Watchable::has_watchers(&self) -> bool`
- Added `Watcher::is_connected(&self) -> bool`

Breaking Changes:
- `Watcher::get` now takes `&mut self` instead of `&self` and returns `Self::Value` instead of `Result<Self::Value, Disconnected>`.
  It will now update the watcher to the latest value internally, so a call to `Watcher::get` in between two `Watcher::poll_updated` calls will potentially have an effect it didn't have before.
  It now also returns the last known state intsead of potentially returning disconnected.
  If you want to know about the disconnected state, use `Watcher::is_connected`.
- `InitializedFut` now implements `Future<Output = T>` instead of `Future<Output = Result<T, Disconnected>>`.
  Should the underlying watchable disconnect, the future will now be pending forever.
