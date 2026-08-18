//! System-level instruction loading for Agent turns.
//!
//! This intentionally loads only explicit project instruction files at the session Worktree root.
//! It does not scan the full project or inject nearby instructions after read
//! tool calls; that behavior remains a separate tool-result concern.

