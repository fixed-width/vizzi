//! Native adapters for the local codebase workspace.
pub mod admission;
mod analysis_jobs;
mod analysis_workers;
pub mod application;
pub mod cache;
pub mod changes;
pub mod companion;
pub mod editor;
pub mod engine;
pub mod host;
mod invalidation;
mod maintenance;
pub mod review;
pub mod review_application;
pub mod review_source;
pub mod server;
pub mod source;
pub mod source_spool;
pub mod syntax_cache;

#[cfg(feature = "qualification")]
pub mod admission_probe;
#[cfg(feature = "qualification")]
pub mod qualification;
#[cfg(feature = "qualification")]
pub mod retention;
#[cfg(feature = "qualification")]
pub mod source_storage_probe;
#[cfg(feature = "qualification")]
pub mod working_set;

#[cfg(feature = "qualification")]
pub mod stream_probe;

#[cfg(feature = "qualification")]
pub mod foreground_maintenance;

#[cfg(feature = "qualification")]
pub mod idle_probe;
