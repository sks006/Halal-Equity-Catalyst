//! Route modules for Equity Catalyst API.

pub mod events;
pub mod executions;
pub mod health;
pub mod oracle;
pub mod policies;
pub mod quotes;
pub mod vaults;
pub mod dbc;

pub use events::*;
pub use executions::*;
pub use health::*;
pub use oracle::*;
pub use policies::*;
pub use quotes::*;
pub use vaults::*;
pub use dbc::*;


