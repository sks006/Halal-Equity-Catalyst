//! Route modules for Equity Catalyst API.

pub mod dbc;
pub mod events;
pub mod executions;
pub mod health;
pub mod market_data;
pub mod oracle;
pub mod policies;
pub mod quotes;
pub mod vaults;

pub use dbc::*;
pub use events::*;
pub use executions::*;
pub use health::*;
pub use market_data::*;
pub use oracle::*;
pub use policies::*;
pub use quotes::*;
pub use vaults::*;
