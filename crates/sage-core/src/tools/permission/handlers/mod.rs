//! Permission handler implementations

mod auto;
mod policy;

pub use auto::{AutoAllowHandler, AutoDenyHandler};
pub use policy::{PermissionPolicy, PolicyHandler};
