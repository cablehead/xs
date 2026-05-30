use scru128::Scru128Id;

/// Two-slot compaction state for a single `<kind>.<name>` lifecycle.
///
/// Tracks the latest known-good `create` (`confirmed`) and the latest
/// untested `create` (`pending`). See ADR 0005 for the state transition
/// table this implements.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Slots {
    confirmed: Option<Scru128Id>,
    pending: Option<Scru128Id>,
}

/// One lifecycle event the state machine consumes. Maps 1:1 to the
/// topic vocabulary in ADR 0005.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// User-appended `create` for this kind/name.
    Create { id: Scru128Id },
    /// Runtime emitted `active`; `source` is the `create`'s id.
    Active { source: Scru128Id },
    /// Runtime emitted `invalid` (failed to init); `source` is the
    /// `create`'s id.
    Invalid { source: Scru128Id },
    /// User-appended `term`. Clears both slots.
    Term,
    /// Runtime emitted any of `fin.error` / `fin.ok` / `fin.term`.
    /// Clears both slots.
    Fin,
    /// Runtime emitted `replaced`. No effect on slots (the replacement's
    /// `active` will overwrite `confirmed` once it arrives).
    Replaced,
    /// Runtime emitted `stopped` (xs.stopping ack). No effect on slots.
    Stopped,
}

/// The dispatcher's decision at threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThresholdPick {
    /// Nothing to start.
    None,
    /// Try `id`. On parse-fail, fall back to `fallback` if `Some`.
    Start {
        id: Scru128Id,
        fallback: Option<Scru128Id>,
    },
}

impl Slots {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one event to the state.
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Create { id } => {
                self.pending = Some(id);
            }
            Event::Active { source } => {
                self.confirmed = Some(source);
                if self.pending == Some(source) {
                    self.pending = None;
                }
            }
            Event::Invalid { source } => {
                if self.pending == Some(source) {
                    self.pending = None;
                }
            }
            Event::Term | Event::Fin => {
                self.confirmed = None;
                self.pending = None;
            }
            Event::Replaced | Event::Stopped => {}
        }
    }

    /// Replay a sequence of events from an empty state.
    pub fn replay<I: IntoIterator<Item = Event>>(events: I) -> Self {
        let mut s = Self::new();
        for e in events {
            s.apply(e);
        }
        s
    }

    /// The dispatcher's pick at threshold:
    ///
    /// * `pending` set: try it; if it parse-fails, fall back to `confirmed`.
    /// * no `pending`, `confirmed` set: start `confirmed` (no fallback).
    /// * both empty: nothing to start.
    pub fn threshold(&self) -> ThresholdPick {
        match (self.pending, self.confirmed) {
            (Some(p), c) => ThresholdPick::Start {
                id: p,
                fallback: c,
            },
            (None, Some(c)) => ThresholdPick::Start {
                id: c,
                fallback: None,
            },
            (None, None) => ThresholdPick::None,
        }
    }

    pub fn confirmed(&self) -> Option<Scru128Id> {
        self.confirmed
    }

    pub fn pending(&self) -> Option<Scru128Id> {
        self.pending
    }
}
