//! Shared two-slot compaction state machine for actor / service / action
//! lifecycles. See ADR 0005 for the topic vocabulary and the algorithm
//! this implements.

mod slots;

#[cfg(test)]
mod tests;

pub use slots::{Event, Slots, ThresholdPick};
