//! Task branch scheduler adapter.
//!
//! Agent owns the Task fact source. This adapter only executes branches of a
//! Task concurrently and reports branch results back to the caller.
//!
//! # Design rules
//! - Do not store Tasks here.
//! - Do not introduce another user-visible task model.
//! - Task Sidechain, local pools, or future extensions can implement this trait as
//!   execution adapters.

