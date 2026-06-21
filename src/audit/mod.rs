#[allow(clippy::module_inception)]
pub mod audit;
pub use audit::{AuditEvent, log};
