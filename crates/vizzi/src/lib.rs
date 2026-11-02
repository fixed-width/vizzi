//! Native adapters for the local codebase workspace.
pub mod application;
pub mod cache;
pub mod changes;
pub mod engine;
pub mod host;
mod invalidation;
pub mod server;
pub mod source;

#[cfg(feature = "qualification")]
pub mod qualification;
