//! Extension domain.
//!
//! This module is the single composition boundary for installable extensions.
//! Extension contributions stay here; the kernel only provides generic
//! registries, policies, pipelines, and events.

pub mod types;
pub mod interfaces;

pub(crate) mod host_view;

pub mod component;

pub mod context;

pub mod installer;

pub mod lifecycle;

pub mod loader;

pub mod models;

pub mod operation_runtime;

pub mod provider_validation;

pub mod resource;

pub mod skills;

pub mod store;

pub use installer::ExtensionInstaller;
pub use lifecycle::ExtensionLifecycle;
pub use loader::ExtensionLoader;
pub use models::*;
pub use provider_validation::*;
pub use store::ExtensionStore;
