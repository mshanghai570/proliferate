pub mod management;
pub mod package;
pub mod registry;
mod theme_json;
pub mod vsix;

#[cfg(test)]
mod vsix_tests;

#[cfg(test)]
mod package_tests;

#[cfg(test)]
pub(crate) mod registry_tests;
