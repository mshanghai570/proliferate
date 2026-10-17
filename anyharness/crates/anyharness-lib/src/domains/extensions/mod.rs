pub mod installer;
pub mod manager;
pub mod manifest;
pub mod model;
mod store;

#[cfg(test)]
mod manager_tests;

#[cfg(test)]
pub(crate) mod installer_tests;
