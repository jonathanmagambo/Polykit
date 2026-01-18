//! HTTP backend for remote cache.

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use tokio::time::sleep;

use crate::error::{Error, Result};

use super::artifact::Artifact;
use super::backend::RemoteCacheBackend;
use super::cache_key::CacheKey;
use super::config::RemoteCacheConfig;

/// HTTP backend for remote cache.
///
/// Supports streaming upload/download, authentication, and retry logic.
pub struct HttpBackend {
    client: Client,
    base_url: String,
    token: Option<String>,
    max_retries: u32,
    retry_delay: Duration,
}

impl HttpBackend {
    /// Creates a new HTTP backend.
    ///
    /// # Arguments
    ///
    /// * `config` - Remote cache configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: &RemoteCacheConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Adapter {
                package: "http-backend".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self {
            client,
            base_url: config.url.trim_end_matches('/').to_string(),
            token: config.token.clone(),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        })
    }

    /// Gets the URL for an artifact.
    fn artifact_url(&self, key: &CacheKey) -> String {
        let key_str = key.as_string();
        format!("{}/v1/artifacts/{}", self.base_url, key_str)
    }


    /// Retries an operation with exponential backoff.
    async fn retry<F, Fut, T>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>> + Send,
    {
        let mut delay = self.retry_delay;
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries {
                        sleep(delay).await;
                        delay *= 2; // Exponential backoff
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Adapter {
            package: "http-backend".to_string(),
            message: "All retry attempts failed".to_string(),
        }))
    }
}

#[async_trait]
impl RemoteCacheBackend for HttpBackend {
    async fn upload_artifact(&self, key: &CacheKey, artifact: &Artifact) -> Result<()> {
        let url = self.artifact_url(key);
        let compressed_data = artifact.compressed_data();

        let client = self.client.clone();
        let token = self.token.clone();
        self.retry(move || {
            let url = url.clone();
            let data = compressed_data.to_vec();
            let client = client.clone();
            let token = token.clone();
            async move {
                let mut builder = client.put(&url).body(data);
                if let Some(ref token) = token {
                    builder = builder.bearer_auth(token);
                }

                let response = builder.send().await.map_err(|e| Error::Adapter {
                    package: "http-backend".to_string(),
                    message: format!("Upload request failed: {}", e),
                })?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    Err(Error::Adapter {
                        package: "http-backend".to_string(),
                        message: format!("Upload failed with status {}: {}", status, text),
                    })
                }
            }
        })
        .await
    }

    async fn fetch_artifact(&self, key: &CacheKey) -> Result<Option<Artifact>> {
        let url = self.artifact_url(key);

        let client = self.client.clone();
        let token = self.token.clone();
        self.retry(move || {
            let url = url.clone();
            let client = client.clone();
            let token = token.clone();
            async move {
                let mut builder = client.get(&url);
                if let Some(ref token) = token {
                    builder = builder.bearer_auth(token);
                }

                let response = builder.send().await.map_err(|e| Error::Adapter {
                    package: "http-backend".to_string(),
                    message: format!("Fetch request failed: {}", e),
                })?;

                match response.status() {
                    status if status.is_success() => {
                        let data = response.bytes().await.map_err(|e| Error::Adapter {
                            package: "http-backend".to_string(),
                            message: format!("Failed to read response body: {}", e),
                        })?;
                        let artifact = Artifact::from_compressed(data.to_vec())?;
                        Ok(Some(artifact))
                    }
                    status if status == reqwest::StatusCode::NOT_FOUND => Ok(None),
                    status => {
                        let text = response.text().await.unwrap_or_default();
                        Err(Error::Adapter {
                            package: "http-backend".to_string(),
                            message: format!("Fetch failed with status {}: {}", status, text),
                        })
                    }
                }
            }
        })
        .await
    }

    async fn has_artifact(&self, key: &CacheKey) -> Result<bool> {
        let url = self.artifact_url(key);

        let client = self.client.clone();
        let token = self.token.clone();
        self.retry(move || {
            let url = url.clone();
            let client = client.clone();
            let token = token.clone();
            async move {
                let mut builder = client.head(&url);
                if let Some(ref token) = token {
                    builder = builder.bearer_auth(token);
                }

                let response = builder.send().await.map_err(|e| Error::Adapter {
                    package: "http-backend".to_string(),
                    message: format!("Exists check failed: {}", e),
                })?;

                match response.status() {
                    status if status.is_success() => Ok(true),
                    status if status == reqwest::StatusCode::NOT_FOUND => Ok(false),
                    status => {
                        let text = response.text().await.unwrap_or_default();
                        Err(Error::Adapter {
                            package: "http-backend".to_string(),
                            message: format!("Exists check failed with status {}: {}", status, text),
                        })
                    }
                }
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_url() {
        let config = RemoteCacheConfig::new("https://cache.example.com");
        let backend = HttpBackend::new(&config).unwrap();

        let key = CacheKey::builder()
            .package_id("test")
            .task_name("build")
            .command("echo")
            .dependency_graph_hash("abc")
            .toolchain_version("node-v20")
            .build()
            .unwrap();

        let url = backend.artifact_url(&key);
        assert!(url.starts_with("https://cache.example.com/v1/artifacts/"));
    }
}
