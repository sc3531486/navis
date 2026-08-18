//! Agent file write guard.
//!
//! Existing-file writes must be based on a prior successful, untruncated read
//! from the same turn transcript. This mirrors the mature agent pattern where
//! file mutation is anchored to observed file facts instead of guesses.

