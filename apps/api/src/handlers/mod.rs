//! HTTP request handlers partitioned according to Single Responsibility Principle (SRP).
//!
//! Each submodule has a single responsibility: processing HTTP requests, validating inputs,
//! invoking domain service or repository abstractions (DIP), and formatting HTTP responses.

pub mod dbc;
pub mod events;
pub mod executions;
pub mod health;
pub mod market_data;
pub mod metrics;
pub mod oracle;
pub mod policies;
pub mod quotes;
pub mod vaults;

pub use dbc::*;
pub use events::*;
pub use executions::*;
pub use health::*;
pub use market_data::*;
pub use metrics::*;
pub use oracle::*;
pub use policies::*;
pub use quotes::*;
pub use vaults::*;
