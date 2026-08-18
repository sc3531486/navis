//! Stable inputs consumed by the Agent tool runtime.
//!
//! The tool catalog only needs the set of tools available in the current
//! mode. Keeping that requirement as a small capability avoids coupling the
//! catalog to the full Agent mode configuration and makes new mode sources
//! straightforward to add.

