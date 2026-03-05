pub mod crates_io;
pub mod pypi;
pub mod npm;

/// Information about the latest version of a package in a registry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistryVersion {
    pub name: String,
    pub version: String,
    pub registry: String,
}

/// Trait for registry clients.
pub trait RegistryClient {
    fn get_latest_version(&self, package_name: &str) -> anyhow::Result<RegistryVersion>;
    fn registry_name(&self) -> &str;
}
