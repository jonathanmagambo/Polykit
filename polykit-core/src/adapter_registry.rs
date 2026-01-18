//! Registry for language adapters to support plugin-like extensibility.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::adapter::LanguageAdapter;
use crate::package::Language;

type AdapterFactory = Box<dyn Fn() -> Box<dyn LanguageAdapter> + Send + Sync>;

/// Global registry for language adapters.
pub struct AdapterRegistry {
    adapters: Mutex<HashMap<String, AdapterFactory>>,
}

impl AdapterRegistry {
    /// Creates a new adapter registry.
    pub fn new() -> Self {
        Self {
            adapters: Mutex::new(HashMap::new()),
        }
    }

    /// Registers an adapter factory for a language.
    ///
    /// # Arguments
    ///
    /// * `language` - The language identifier (e.g., "js", "rust")
    /// * `factory` - A function that creates an instance of the adapter
    pub fn register<F>(&self, language: &str, factory: F)
    where
        F: Fn() -> Box<dyn LanguageAdapter> + Send + Sync + 'static,
    {
        if let Ok(mut adapters) = self.adapters.lock() {
            adapters.insert(language.to_string(), Box::new(factory));
        }
    }

    /// Gets an adapter for a language.
    ///
    /// Returns `None` if no adapter is registered for the language.
    pub fn get(&self, language: &str) -> Option<Box<dyn LanguageAdapter>> {
        self.adapters
            .lock()
            .ok()
            .and_then(|adapters| adapters.get(language).map(|factory| factory()))
    }

    /// Gets an adapter for a Language enum.
    pub fn get_for_language(&self, language: &Language) -> Option<Box<dyn LanguageAdapter>> {
        self.get(language.as_str())
    }

    /// Lists all registered languages.
    pub fn registered_languages(&self) -> Vec<String> {
        self.adapters
            .lock()
            .ok()
            .map(|adapters| adapters.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
