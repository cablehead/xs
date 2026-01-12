// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

pub mod report;

use crate::{id::SegmentId, BlobCache, Compressor, ValueLog};

/// GC strategy
#[allow(clippy::module_name_repetitions)]
pub trait GcStrategy<BC: BlobCache, C: Compressor + Clone> {
    /// Picks segments based on a predicate.
    fn pick(&self, value_log: &ValueLog<BC, C>) -> Vec<SegmentId>;
}

/// Picks segments that have a certain percentage of stale blobs
pub struct StaleThresholdStrategy(f32);

impl StaleThresholdStrategy {
    /// Creates a new strategy with the given threshold.
    ///
    /// # Panics
    ///
    /// Panics if the ratio is invalid.
    #[must_use]
    pub fn new(ratio: f32) -> Self {
        assert!(
            ratio.is_finite() && ratio.is_sign_positive(),
            "invalid stale ratio"
        );
        Self(ratio.min(1.0))
    }
}

impl<BC: BlobCache, C: Compressor + Clone> GcStrategy<BC, C> for StaleThresholdStrategy {
    fn pick(&self, value_log: &ValueLog<BC, C>) -> Vec<SegmentId> {
        value_log
            .manifest
            .segments
            .read()
            .expect("lock is poisoned")
            .values()
            .filter(|x| x.stale_ratio() > self.0)
            .map(|x| x.id)
            .collect::<Vec<_>>()
    }
}

/// Tries to find a least-effort-selection of segments to merge to reach a certain space amplification
pub struct SpaceAmpStrategy(f32);

impl SpaceAmpStrategy {
    /// Creates a new strategy with the given space amp factor.
    ///
    /// # Panics
    ///
    /// Panics if the space amp factor is < 1.0.
    #[must_use]
    pub fn new(ratio: f32) -> Self {
        assert!(ratio >= 1.0, "invalid space amp ratio");
        Self(ratio)
    }
}

impl<BC: BlobCache, C: Compressor + Clone> GcStrategy<BC, C> for SpaceAmpStrategy {
    #[allow(clippy::cast_precision_loss, clippy::significant_drop_tightening)]
    fn pick(&self, value_log: &ValueLog<BC, C>) -> Vec<SegmentId> {
        let space_amp_target = self.0;
        let current_space_amp = value_log.space_amp();

        if current_space_amp < space_amp_target {
            log::trace!("Space amp is <= target {space_amp_target}, nothing to do");
            vec![]
        } else {
            log::debug!("Selecting segments to GC, space_amp_target={space_amp_target}");

            let lock = value_log
                .manifest
                .segments
                .read()
                .expect("lock is poisoned");

            let mut segments = lock
                .values()
                .filter(|x| x.stale_ratio() > 0.0)
                .collect::<Vec<_>>();

            // Sort by stale ratio descending
            segments.sort_by(|a, b| {
                b.stale_ratio()
                    .partial_cmp(&a.stale_ratio())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut selection = vec![];

            let mut total_bytes = value_log.manifest.total_bytes();
            let mut stale_bytes = value_log.manifest.stale_bytes();

            for segment in segments {
                let segment_stale_bytes = segment.gc_stats.stale_bytes();
                stale_bytes -= segment_stale_bytes;
                total_bytes -= segment_stale_bytes;

                selection.push(segment.id);

                let space_amp_after_gc =
                    total_bytes as f32 / (total_bytes as f32 - stale_bytes as f32);

                log::debug!(
                    "Selected segment #{} for GC: will reduce space amp to {space_amp_after_gc}",
                    segment.id
                );

                if space_amp_after_gc <= space_amp_target {
                    break;
                }
            }

            selection
        }
    }
}
