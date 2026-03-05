use super::{RegistryClient, RegistryVersion};
use crate::http_client::HttpClient;

pub struct NpmClient {
    client: HttpClient,
    base_url: String,
}

impl NpmClient {
    pub fn new(client: HttpClient) -> Self {
        Self {
            client,
            base_url: "https://registry.npmjs.org".to_string(),
        }
    }

    pub fn with_base_url(client: HttpClient, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[derive(serde::Deserialize)]
struct NpmResponse {
    #[serde(rename = "dist-tags")]
    dist_tags: NpmDistTags,
}

#[derive(serde::Deserialize)]
struct NpmDistTags {
    latest: String,
}

impl RegistryClient for NpmClient {
    fn get_latest_version(&self, package_name: &str) -> anyhow::Result<RegistryVersion> {
        let url = format!("{}/{}", self.base_url, package_name);
        let response: NpmResponse = self.client.get_json(&url)?;
        Ok(RegistryVersion {
            name: package_name.to_string(),
            version: response.dist_tags.latest,
            registry: "npm".to_string(),
        })
    }

    fn registry_name(&self) -> &str {
        "npm"
    }
}
