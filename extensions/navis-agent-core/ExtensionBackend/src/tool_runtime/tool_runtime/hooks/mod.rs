//! Host-owned Agent tool hook runner.
//!
//! Extension manifests can declare lifecycle hooks, but the host owns execution.
//! The first supported action is a deterministic PreToolUse deny/continue
//! decision, evaluated without loading extension JavaScript.

