use super::{RegistryClient, RegistryVersion};
use crate::http_client::HttpClient;

pub struct CratesIoClient {
    client: HttpClient,
    base_url: String,
}

impl CratesIoClient {
    pub fn new(client: HttpClient) -> Self {
        Self {
            client,
            base_url: "https://crates.io/api/v1".to_string(),
        }
    }

    pub fn with_base_url(client: HttpClient, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[derive(serde::Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    krate: CrateInfo,
}

#[derive(serde::Deserialize)]
struct CrateInfo {
    max_version: String,
}

impl RegistryClient for CratesIoClient {
    fn get_latest_version(&self, package_name: &str) -> anyhow::Result<RegistryVersion> {
        let url = format!("{}/crates/{}", self.base_url, package_name);
        let response: CrateResponse = self.client.get_json(&url)?;
        Ok(RegistryVersion {
            name: package_name.to_string(),
            version: response.krate.max_version,
            registry: "crates.io".to_string(),
        })
    }

    fn registry_name(&self) -> &str {
        "crates.io"
    }
}
