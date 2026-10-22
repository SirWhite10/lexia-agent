//! Plugins: capabilities the agent does not have built in.
//!
//! The order of this module is the order of the trust argument.
//!
//! - [`capabilities`] — what a plugin may *ask* for. A closed enum, so a typo
//!   fails loudly instead of quietly granting nothing.
//! - [`consent`] — what it may *have*. Deny-by-default, per agent, revocable,
//!   and revocation reports what to deactivate immediately.
//! - [`store`] — what is *installed*. Content-addressed generations, exactly one
//!   active, promoted only as a deliberate act.
//! - [`runtime`] — what it may *do*, once loaded. Wasmtime, with the ceilings
//!   the manifest asked for and the imports the consent ledger granted.
//!
//! The separation is the point: a plugin declares, an operator consents, the
//! host enforces. Collapsing any two of them into one record is how a plugin
//! system ends up granting itself authority.

pub mod capabilities;
pub mod consent;
pub mod runtime;
pub mod store;

#[cfg(test)]
mod tests;

pub use capabilities::{Capability, Manifest, Resources};
pub use consent::{ConsentLedger, Decision};
pub use runtime::{LoadedPlugin, PluginError, PluginHost};
pub use store::{Generation, Promotion, PromotionError, Registry, RegistryEntry};