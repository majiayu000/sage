//! Permission handler implementations

mod auto;
#[cfg(test)]
mod policy;

pub use auto::{AutoAllowHandler, AutoDenyHandler};
#[cfg(test)]
pub use policy::{PermissionPolicy, PolicyHandler};
