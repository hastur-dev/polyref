use super::{RegistryClient, RegistryVersion};
use crate::http_client::HttpClient;

pub struct PypiClient {
    client: HttpClient,
    base_url: String,
}

impl PypiClient {
    pub fn new(client: HttpClient) -> Self {
        Self {
            client,
            base_url: "https://pypi.org/pypi".to_string(),
        }
    }

    pub fn with_base_url(client: HttpClient, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[derive(serde::Deserialize)]
struct PypiResponse {
    info: PypiInfo,
}

#[derive(serde::Deserialize)]
struct PypiInfo {
    version: String,
}

impl RegistryClient for PypiClient {
    fn get_latest_version(&self, package_name: &str) -> anyhow::Result<RegistryVersion> {
        let url = format!("{}/{}/json", self.base_url, package_name);
        let response: PypiResponse = self.client.get_json(&url)?;
        Ok(RegistryVersion {
            name: package_name.to_string(),
            version: response.info.version,
            registry: "pypi".to_string(),
        })
    }

    fn registry_name(&self) -> &str {
        "pypi"
    }
}
