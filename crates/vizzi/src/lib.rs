//! Native adapters for the local codebase workspace.
pub mod admission;
pub mod application;
pub mod cache;
pub mod changes;
pub mod engine;
pub mod host;
mod invalidation;
pub mod server;
pub mod source;
pub mod syntax_cache;

#[cfg(feature = "qualification")]
pub mod admission_probe;
#[cfg(feature = "qualification")]
pub mod qualification;
#[cfg(feature = "qualification")]
pub mod retention;
#[cfg(feature = "qualification")]
pub mod working_set;
