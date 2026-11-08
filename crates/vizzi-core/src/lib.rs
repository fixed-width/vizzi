//! Immutable source-backed models and analysis, independent of native hosts.

pub mod analysis;
pub mod c_family;
pub mod calls;
pub mod capture;
mod captured_source;
pub mod coloring;
pub mod comparison;
pub mod discovery;
pub mod investigation;
pub mod model;
pub mod policy;
pub mod projection;
pub mod publication;
pub mod python;
pub mod query;
pub mod rust;
mod rust_calls;
pub mod structure;
mod syntax;
mod syntax_state;
pub mod text_diff;
