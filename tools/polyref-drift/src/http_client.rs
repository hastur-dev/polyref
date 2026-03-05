/// HTTP client with optional proxy support.
pub struct HttpClient {
    proxy: Option<String>,
}

impl HttpClient {
    pub fn new(proxy: Option<String>) -> Self {
        Self { proxy }
    }

    /// Perform a GET request and return the response body as a string.
    pub fn get(&self, url: &str) -> anyhow::Result<String> {
        let agent = if let Some(proxy) = &self.proxy {
            ureq::AgentBuilder::new()
                .proxy(ureq::Proxy::new(proxy)?)
                .build()
        } else {
            ureq::Agent::new()
        };

        let response = agent.get(url).call()?;
        Ok(response.into_string()?)
    }

    /// Perform a GET and parse JSON response.
    pub fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> anyhow::Result<T> {
        let body = self.get(url)?;
        Ok(serde_json::from_str(&body)?)
    }
}
