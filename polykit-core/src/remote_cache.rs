//! Remote caching support for shared cache across CI/CD and team members.

use std::path::Path;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct RemoteCacheConfig {
    pub url: String,
    pub token: Option<String>,
}

pub trait RemoteCacheBackend: Send + Sync {
    fn upload(&self, key: &str, data: &[u8]) -> Result<()>;
    fn download(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn exists(&self, key: &str) -> Result<bool>;
}

pub struct LocalRemoteCache;

impl RemoteCacheBackend for LocalRemoteCache {
    fn upload(&self, _key: &str, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn download(&self, _key: &str) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn exists(&self, _key: &str) -> Result<bool> {
        Ok(false)
    }
}

pub struct RemoteCache {
    backend: Box<dyn RemoteCacheBackend>,
    #[allow(dead_code)]
    config: RemoteCacheConfig,
}

impl RemoteCache {
    pub fn new(backend: Box<dyn RemoteCacheBackend>, config: RemoteCacheConfig) -> Self {
        Self { backend, config }
    }

    pub fn local() -> Self {
        Self {
            backend: Box::new(LocalRemoteCache),
            config: RemoteCacheConfig {
                url: "local://".to_string(),
                token: None,
            },
        }
    }

    pub fn upload(&self, key: &str, data: &[u8]) -> Result<()> {
        self.backend.upload(key, data)
    }

    pub fn download(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.backend.download(key)
    }

    pub fn exists(&self, key: &str) -> Result<bool> {
        self.backend.exists(key)
    }

    pub fn cache_key_from_path(path: &Path) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
