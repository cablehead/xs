use super::{Event, Slots, ThresholdPick};
use scru128::Scru128Id;

fn id() -> Scru128Id {
    scru128::new()
}

// ----- single-transition unit tests (one per row of the state table) -----

#[test]
fn create_sets_pending_leaves_confirmed() {
    let a = id();
    let mut s = Slots::new();
    s.apply(Event::Create { id: a });
    assert_eq!(s.pending(), Some(a));
    assert_eq!(s.confirmed(), None);
}

#[test]
fn active_matching_pending_promotes_and_clears_pending() {
    let a = id();
    let mut s = Slots::new();
    s.apply(Event::Create { id: a });
    s.apply(Event::Active { source: a });
    assert_eq!(s.confirmed(), Some(a));
    assert_eq!(s.pending(), None);
}

#[test]
fn active_not_matching_pending_advances_confirmed_only() {
    let a = id();
    let b = id();
    let mut s = Slots::new();
    s.apply(Event::Create { id: b });
    // Active for a different (older) create; pending stays.
    s.apply(Event::Active { source: a });
    assert_eq!(s.confirmed(), Some(a));
    assert_eq!(s.pending(), Some(b));
}

#[test]
fn invalid_matching_pending_clears_it() {
    let a = id();
    let mut s = Slots::new();
    s.apply(Event::Create { id: a });
    s.apply(Event::Invalid { source: a });
    assert_eq!(s.pending(), None);
    assert_eq!(s.confirmed(), None);
}

#[test]
fn invalid_not_matching_pending_has_no_effect() {
    let a = id();
    let b = id();
    let mut s = Slots::new();
    s.apply(Event::Create { id: b });
    // Stale invalid for an older create; pending must survive.
    s.apply(Event::Invalid { source: a });
    assert_eq!(s.pending(), Some(b));
}

#[test]
fn invalid_does_not_clear_confirmed() {
    let a = id();
    let b = id();
    let mut s = Slots::new();
    s.apply(Event::Create { id: a });
    s.apply(Event::Active { source: a });
    s.apply(Event::Create { id: b });
    s.apply(Event::Invalid { source: b });
    assert_eq!(s.confirmed(), Some(a));
    assert_eq!(s.pending(), None);
}

#[test]
fn term_clears_both() {
    let a = id();
    let b = id();
    let mut s = Slots::new();
    s.apply(Event::Create { id: a });
    s.apply(Event::Active { source: a });
    s.apply(Event::Create { id: b });
    s.apply(Event::Term);
    assert_eq!(s.confirmed(), None);
    assert_eq!(s.pending(), None);
}

#[test]
fn fin_clears_both() {
    let a = id();
    let mut s = Slots::new();
    s.apply(Event::Create { id: a });
    s.apply(Event::Active { source: a });
    s.apply(Event::Fin);
    assert_eq!(s.confirmed(), None);
    assert_eq!(s.pending(), None);
}

#[test]
fn replaced_has_no_effect() {
    let a = id();
    let before = Slots::replay([Event::Create { id: a }, Event::Active { source: a }]);
    let mut after = before.clone();
    after.apply(Event::Replaced);
    assert_eq!(before, after);
}

#[test]
fn stopped_has_no_effect() {
    let a = id();
    let before = Slots::replay([Event::Create { id: a }, Event::Active { source: a }]);
    let mut after = before.clone();
    after.apply(Event::Stopped);
    assert_eq!(before, after);
}

// ----- threshold pick -----

#[test]
fn threshold_empty_picks_none() {
    let s = Slots::new();
    assert_eq!(s.threshold(), ThresholdPick::None);
}

#[test]
fn threshold_only_confirmed_starts_without_fallback() {
    let a = id();
    let s = Slots::replay([Event::Create { id: a }, Event::Active { source: a }]);
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: a,
            fallback: None
        }
    );
}

#[test]
fn threshold_only_pending_tries_with_no_fallback() {
    let a = id();
    let s = Slots::replay([Event::Create { id: a }]);
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: a,
            fallback: None
        }
    );
}

#[test]
fn threshold_pending_and_confirmed_tries_pending_with_confirmed_as_fallback() {
    let a = id();
    let b = id();
    let s = Slots::replay([
        Event::Create { id: a },
        Event::Active { source: a },
        Event::Create { id: b },
    ]);
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: b,
            fallback: Some(a),
        }
    );
}

// ----- invariant scenarios -----
//
// Names map to the invariants in ADR 0005:
//   inv1_*, I1 Stop persistence
//   inv2_*, I2 Run persistence
//   inv3_*, I3 Hot-replace fallback
//   inv7_*, I7 Server-shutdown invisibility

#[test]
fn inv1_term_persists_across_replay_into_fresh_slots() {
    let a = id();
    let events = vec![
        Event::Create { id: a },
        Event::Active { source: a },
        Event::Term,
    ];
    let s = Slots::replay(events);
    assert_eq!(s.threshold(), ThresholdPick::None);
}

#[test]
fn inv1_fin_persists_across_replay_into_fresh_slots() {
    let a = id();
    let events = vec![
        Event::Create { id: a },
        Event::Active { source: a },
        Event::Fin,
    ];
    let s = Slots::replay(events);
    assert_eq!(s.threshold(), ThresholdPick::None);
}

#[test]
fn inv2_active_without_terminal_starts_at_threshold() {
    let a = id();
    let s = Slots::replay([Event::Create { id: a }, Event::Active { source: a }]);
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: a,
            fallback: None
        }
    );
}

#[test]
fn inv3_hot_replace_broken_replacement_falls_back_to_confirmed() {
    let a = id();
    let b = id();
    // create_1, active_1, create_2, invalid_2 -> fall back to create_1.
    let s = Slots::replay([
        Event::Create { id: a },
        Event::Active { source: a },
        Event::Create { id: b },
        Event::Invalid { source: b },
    ]);
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: a,
            fallback: None,
        }
    );
}

#[test]
fn inv3_hot_replace_untested_replacement_tries_pending_with_fallback() {
    let a = id();
    let b = id();
    // create_1, active_1, create_2 (no ack, xs died) -> try b, fall back to a.
    let s = Slots::replay([
        Event::Create { id: a },
        Event::Active { source: a },
        Event::Create { id: b },
    ]);
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: b,
            fallback: Some(a),
        }
    );
}

#[test]
fn inv3_hot_replace_transition_window_preserves_confirmed() {
    let a = id();
    let b = id();
    // create_1, active_1, create_2, replaced_1 (xs died before active_2).
    // confirmed must still be a for the fallback to work.
    let s = Slots::replay([
        Event::Create { id: a },
        Event::Active { source: a },
        Event::Create { id: b },
        Event::Replaced,
    ]);
    assert_eq!(s.confirmed(), Some(a));
    assert_eq!(s.pending(), Some(b));
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: b,
            fallback: Some(a),
        }
    );
}

#[test]
fn inv3_rolling_untested_overwrites_earlier_pending() {
    let a = id();
    let b = id();
    let c = id();
    // create_1, active_1, create_2, invalid_2, create_3 -> pending=c, confirmed=a.
    let s = Slots::replay([
        Event::Create { id: a },
        Event::Active { source: a },
        Event::Create { id: b },
        Event::Invalid { source: b },
        Event::Create { id: c },
    ]);
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: c,
            fallback: Some(a),
        }
    );
}

/// I5 Distinct exit categories at the algorithm layer: every Event variant
/// the dispatcher might apply to Slots is a distinct enum case. The runtime
/// has no way to confuse one variant with another, which is what the
/// invariant asserts in observable form (one topic per category).
///
/// I5 is also tested at the dispatcher layer: each kind's event_from_frame
/// translator covers every event tag in the vocabulary -- see
/// inv5_*_topic_tags_are_complete in the per-kind tests.
#[test]
fn inv5_event_variants_cover_the_vocabulary() {
    let a = id();
    let touched = [
        Event::Create { id: a },
        Event::Term,
        Event::Active { source: a },
        Event::Invalid { source: a },
        Event::Fin,
        Event::Replaced,
        Event::Stopped,
    ];
    // The compile-time guarantee is what the property hangs on: this
    // assertion exists so that an attempt to silently merge two variants
    // would fail to compile (the match in Slots::apply is exhaustive). The
    // body just touches every variant in turn.
    let mut s = Slots::new();
    for ev in touched {
        s.apply(ev);
    }
    let _ = s.threshold();
}

#[test]
fn inv7_stopped_does_not_clear_slots() {
    let a = id();
    let s = Slots::replay([
        Event::Create { id: a },
        Event::Active { source: a },
        Event::Stopped,
    ]);
    assert_eq!(
        s.threshold(),
        ThresholdPick::Start {
            id: a,
            fallback: None,
        }
    );
}
